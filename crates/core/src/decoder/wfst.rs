use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::roman_lookup::{char_ngrams, roman_search_variants, LegacyData, RankedLexicon, RankedLexiconEntry};

use super::{
    elapsed_decode_us, start_decode_timer, DecodeCandidate, DecodeFailure, DecodeRequest, DecodeResult, DecodeSegment,
    Decoder, DecoderConfig,
};

const MIN_SPAN_LEN: usize = 3;
const CHUNK_EDIT_FLOOR: f64 = 0.46;
const SEGMENT_LIMIT_PER_START: usize = 12;

// This decoder is WFST-style rather than a compiled finite-state transducer. It
// builds span candidates from lexicon retrieval signals and runs a beam search
// with weighted chunk, segmentation, language-model, POS, and history scores.
// Keep score components separated so shadow/hybrid modes can explain ranking
// changes without platform adapters knowing decoder internals.

#[derive(Clone, Debug)]
struct RetrievalSignals {
    raw_exact_hit: bool,
    exact_hit: bool,
    alias_hit: bool,
    gram_hits: usize,
}

#[derive(Clone, Debug)]
struct SpanCandidate {
    start: usize,
    end: usize,
    input: String,
    output: String,
    recovered_roman: String,
    source_rank: usize,
    first_tag: Option<String>,
    last_tag: Option<String>,
    score: i32,
    score_bps: u16,
    edit_similarity: f64,
}

#[derive(Clone, Debug, Default)]
struct ScoreBreakdown {
    chunk: i32,
    segmentation: i32,
    lm: i32,
    pos: i32,
    history: i32,
}

#[derive(Clone, Debug, Default)]
struct BeamItem {
    position: usize,
    spans: Vec<SpanCandidate>,
    recovered_romans: Vec<String>,
    words: Vec<String>,
    last_tags: Vec<Option<String>>,
    scores: ScoreBreakdown,
}

impl ScoreBreakdown {
    fn total(&self, config: &DecoderConfig) -> i32 {
        self.chunk * config.chunk_score_weight
            + self.segmentation * config.segmentation_score_weight
            + self.lm * config.lm_score_weight
            + self.pos * config.pos_score_weight
            + self.history * config.history_score_weight
    }
}

impl BeamItem {
    fn total_score(&self, config: &DecoderConfig) -> i32 {
        self.scores.total(config)
    }

    fn final_text(&self) -> String {
        self.words.concat()
    }
}

pub(crate) struct WfstDecoder {
    data: Arc<LegacyData>,
    config: DecoderConfig,
}

impl WfstDecoder {
    pub(crate) fn new(data: Arc<LegacyData>, config: DecoderConfig) -> Self {
        Self { data, config }
    }

    fn decode_with_beam(&self, request: &DecodeRequest<'_>, started_at: super::DecodeTimer) -> Vec<BeamItem> {
        let chars = request.composer.normalized.chars().collect::<Vec<_>>();
        if chars.is_empty() {
            return Vec::new();
        }
        let exact_full_span = self.decode_exact_full_span(request, &chars, started_at);

        if self.config.interactive_mode && self.has_anchor_chunks(request) {
            let anchored = self.decode_with_anchors(request, &chars, started_at);
            if !anchored.is_empty() {
                let mut finals = anchored;
                finals.extend(exact_full_span);
                finals.sort_by(|left, right| compare_beam_items(left, right, &self.config));
                finals.truncate(self.config.max_candidates.max(self.config.beam_width));
                return finals;
            }
        }

        let per_start = match self.build_span_lattice(request, &chars, started_at) {
            Some(per_start) => per_start,
            None => return self.decode_with_anchors(request, &chars, started_at),
        };
        let mut beams = vec![Vec::<BeamItem>::new(); chars.len() + 1];
        beams[0].push(BeamItem::default());

        for start in 0..chars.len() {
            if self.over_budget(started_at) {
                break;
            }
            if beams[start].is_empty() || per_start[start].is_empty() {
                continue;
            }

            let current = beams[start].clone();
            for beam in current {
                for candidate in &per_start[start] {
                    let next = self.extend_beam(beam.clone(), candidate, request);
                    let bucket = &mut beams[candidate.end];
                    bucket.push(next);
                    prune_beam(bucket, self.config.beam_width, &self.config);
                }
            }
        }

        let mut finals = beams[chars.len()]
            .iter()
            .cloned()
            .map(|mut item| {
                item.scores.history += final_history_score(request.history, &item.final_text());
                item
            })
            .collect::<Vec<_>>();
        finals.extend(exact_full_span);
        finals.extend(self.decode_with_anchors(request, &chars, started_at));
        finals.sort_by(|left, right| compare_beam_items(left, right, &self.config));
        finals.truncate(self.config.max_candidates.max(self.config.beam_width));
        finals
    }

    fn decode_exact_full_span(
        &self,
        request: &DecodeRequest<'_>,
        chars: &[char],
        started_at: super::DecodeTimer,
    ) -> Vec<BeamItem> {
        if self.over_budget(started_at) {
            return Vec::new();
        }
        let span = chars.iter().collect::<String>();
        if !self.data.ranked().exact_index.contains_key(span.as_str()) {
            return Vec::new();
        }

        self.rerank_span_candidates(0, chars.len(), &span)
            .into_iter()
            .map(|candidate| {
                let mut beam = self.extend_beam(BeamItem::default(), &candidate, request);
                beam.scores.segmentation += 30_000;
                beam.scores.history += final_history_score(request.history, &beam.final_text());
                beam
            })
            .collect()
    }

    fn decode_with_anchors(
        &self,
        request: &DecodeRequest<'_>,
        chars: &[char],
        started_at: super::DecodeTimer,
    ) -> Vec<BeamItem> {
        if let Some(ranges) = self.preferred_anchor_ranges(request, chars.len()) {
            let mut beam = BeamItem::default();
            for (start, end) in ranges {
                if self.over_budget(started_at) {
                    return Vec::new();
                }
                let span = chars[start..end].iter().collect::<String>();
                let Some(candidate) = self.rerank_span_candidates(start, end, &span).into_iter().next() else {
                    return Vec::new();
                };
                beam = self.extend_beam(beam, &candidate, request);
            }
            beam.scores.segmentation += 24_000;
            beam.scores.history += final_history_score(request.history, &beam.final_text());
            return vec![beam];
        }

        if !self.config.interactive_mode {
            return Vec::new();
        }

        let mut strong_ranges = request
            .composer
            .chunks
            .iter()
            .filter_map(|chunk| {
                let len = chunk.end.saturating_sub(chunk.start);
                (len >= 4).then_some((chunk.start, chunk.end))
            })
            .collect::<Vec<_>>();
        if strong_ranges.is_empty() {
            return Vec::new();
        }

        if strong_ranges[0].0 <= MIN_SPAN_LEN {
            strong_ranges[0].0 = 0;
        }
        for index in 1..strong_ranges.len() {
            let gap = strong_ranges[index].0.saturating_sub(strong_ranges[index - 1].1);
            if gap > 0 && gap <= MIN_SPAN_LEN {
                strong_ranges[index].0 = strong_ranges[index - 1].1;
            }
        }
        if chars
            .len()
            .saturating_sub(strong_ranges.last().map(|(_, end)| *end).unwrap_or(0))
            <= MIN_SPAN_LEN
        {
            if let Some(last) = strong_ranges.last_mut() {
                last.1 = chars.len();
            }
        }

        let mut ranges = Vec::<(usize, usize)>::new();
        let mut cursor = 0usize;
        for (start, end) in strong_ranges {
            if cursor < start {
                ranges.push((cursor, start));
            }
            ranges.push((start, end));
            cursor = end;
        }
        if cursor < chars.len() {
            ranges.push((cursor, chars.len()));
        }

        let mut beam = BeamItem::default();
        for (start, end) in ranges {
            if self.over_budget(started_at) {
                return Vec::new();
            }
            if end.saturating_sub(start) < MIN_SPAN_LEN {
                return Vec::new();
            }
            let span = chars[start..end].iter().collect::<String>();
            let Some(candidate) = self.rerank_span_candidates(start, end, &span).into_iter().next() else {
                return Vec::new();
            };
            beam = self.extend_beam(beam, &candidate, request);
        }
        beam.scores.segmentation += 12_000;
        beam.scores.history += final_history_score(request.history, &beam.final_text());
        vec![beam]
    }

    fn build_span_lattice(
        &self,
        request: &DecodeRequest<'_>,
        chars: &[char],
        started_at: super::DecodeTimer,
    ) -> Option<Vec<Vec<SpanCandidate>>> {
        let mut per_start = vec![Vec::<SpanCandidate>::new(); chars.len()];
        let preferred_ends = self.preferred_lattice_ends(request, chars.len());
        for start in 0..chars.len() {
            if self.over_budget(started_at) {
                return None;
            }
            let remaining = chars.len().saturating_sub(start);
            if remaining < MIN_SPAN_LEN {
                continue;
            }
            let mut best_by_signature = HashMap::<(usize, String), SpanCandidate>::new();
            let lengths = if let Some(ends) = preferred_ends.get(&start) {
                ends.iter()
                    .copied()
                    .filter(|end| *end > start)
                    .map(|end| end - start)
                    .collect::<Vec<_>>()
            } else {
                let max_len = self.config.beam_max_span_len.min(remaining);
                let mut lengths = (MIN_SPAN_LEN..=max_len).rev().collect::<Vec<_>>();
                lengths.truncate(self.config.beam_lengths_per_start.max(1));
                lengths
            };

            for len in lengths {
                let end = start + len;
                let span = chars[start..end].iter().collect::<String>();
                for candidate in self.rerank_span_candidates(start, end, &span) {
                    let key = (candidate.end, candidate.output.clone());
                    match best_by_signature.get(&key) {
                        Some(current) if current.score >= candidate.score => {}
                        _ => {
                            best_by_signature.insert(key, candidate);
                        }
                    }
                }
            }

            let mut candidates = best_by_signature.into_values().collect::<Vec<_>>();
            candidates.sort_by(|left, right| compare_span_candidates(left, right));
            candidates.truncate(SEGMENT_LIMIT_PER_START.min(self.config.beam_retrieval_shortlist.max(1)));
            per_start[start] = candidates;
        }
        Some(per_start)
    }

    fn rerank_span_candidates(&self, start: usize, end: usize, span: &str) -> Vec<SpanCandidate> {
        let search_keys = roman_search_variants(span);
        if search_keys.is_empty() {
            return Vec::new();
        }

        let raw_key = search_keys.first().map(String::as_str).unwrap_or(span);
        let retrieved = self.retrieve_candidates(&search_keys, raw_key);
        if retrieved.is_empty() {
            return Vec::new();
        }
        let has_raw_exact_hit = retrieved.iter().any(|(_, signals)| signals.raw_exact_hit);
        let has_exact_hit = retrieved.iter().any(|(_, signals)| signals.exact_hit);

        let mut best_by_output = HashMap::<String, SpanCandidate>::new();
        for (entry_index, signals) in retrieved {
            if has_raw_exact_hit && !signals.raw_exact_hit {
                continue;
            }
            if has_exact_hit && !signals.exact_hit {
                continue;
            }
            let entry = &self.data.ranked().entries[entry_index];
            if let Some(candidate) = self.score_span_candidate(start, end, span, entry, &search_keys, &signals) {
                match best_by_output.get(candidate.output.as_str()) {
                    Some(current) if !compare_span_candidates(current, &candidate).is_gt() => {}
                    _ => {
                        best_by_output.insert(candidate.output.clone(), candidate);
                    }
                }
            }
        }

        let mut ranked = best_by_output.into_values().collect::<Vec<_>>();
        ranked.sort_by(|left, right| compare_span_candidates(left, right));
        ranked.truncate(self.config.beam_candidates_per_span);
        ranked
    }

    fn retrieve_candidates(&self, search_keys: &[String], raw_key: &str) -> Vec<(usize, RetrievalSignals)> {
        let lexicon = self.data.ranked();
        if let Some(ids) = lexicon.exact_index.get(raw_key) {
            let mut ranked = ids
                .iter()
                .copied()
                .map(|entry_index| {
                    (
                        entry_index,
                        RetrievalSignals {
                            raw_exact_hit: true,
                            exact_hit: true,
                            alias_hit: false,
                            gram_hits: 0,
                        },
                    )
                })
                .collect::<Vec<_>>();
            ranked.sort_by_key(|(entry_index, _)| Reverse(lexicon.entries[*entry_index].frequency));
            ranked.truncate(self.config.beam_retrieval_shortlist.max(1));
            return ranked;
        }
        let mut exact_signals = HashMap::<usize, RetrievalSignals>::new();
        let mut alias_only = HashMap::<usize, RetrievalSignals>::new();

        for key in search_keys {
            if let Some(ids) = lexicon.exact_index.get(key) {
                for &entry_index in ids {
                    let signal = exact_signals.entry(entry_index).or_insert_with(|| RetrievalSignals {
                        raw_exact_hit: false,
                        exact_hit: false,
                        alias_hit: false,
                        gram_hits: 0,
                    });
                    if key == raw_key {
                        signal.raw_exact_hit = true;
                    }
                    signal.exact_hit = true;
                }
            }
            if let Some(ids) = lexicon.alias_index.get(key) {
                for &entry_index in ids {
                    alias_only
                        .entry(entry_index)
                        .or_insert_with(|| RetrievalSignals {
                            raw_exact_hit: false,
                            exact_hit: false,
                            alias_hit: true,
                            gram_hits: 0,
                        })
                        .alias_hit = true;
                }
            }
        }

        if self.config.interactive_mode && !exact_signals.is_empty() {
            let mut ranked = exact_signals.into_iter().collect::<Vec<_>>();
            ranked.sort_by(|left, right| compare_retrieval_hits(left, right, lexicon));
            ranked.truncate(self.config.beam_retrieval_shortlist.max(1));
            return ranked;
        }

        if self.config.interactive_mode && raw_key.chars().count() >= 4 && !alias_only.is_empty() {
            let mut ranked = alias_only.into_iter().collect::<Vec<_>>();
            ranked.sort_by(|left, right| compare_retrieval_hits(left, right, lexicon));
            ranked.truncate(self.config.beam_retrieval_shortlist.max(1));
            return ranked;
        }

        let mut signals = HashMap::<usize, RetrievalSignals>::new();

        for key in search_keys {
            if let Some(ids) = lexicon.exact_index.get(key) {
                for &entry_index in ids {
                    signals.entry(entry_index).or_insert_with(|| RetrievalSignals {
                        raw_exact_hit: false,
                        exact_hit: false,
                        alias_hit: false,
                        gram_hits: 0,
                    });
                    if key == raw_key {
                        signals.get_mut(&entry_index).expect("entry inserted").raw_exact_hit = true;
                    }
                    signals.get_mut(&entry_index).expect("entry inserted").exact_hit = true;
                }
            }
            if let Some(ids) = lexicon.alias_index.get(key) {
                for &entry_index in ids {
                    signals
                        .entry(entry_index)
                        .or_insert_with(|| RetrievalSignals {
                            raw_exact_hit: false,
                            exact_hit: false,
                            alias_hit: false,
                            gram_hits: 0,
                        })
                        .alias_hit = true;
                }
            }
            for gram in char_ngrams(key, 2) {
                if let Some(ids) = lexicon.gram_index.get(&gram) {
                    for &entry_index in ids {
                        signals
                            .entry(entry_index)
                            .or_insert_with(|| RetrievalSignals {
                                raw_exact_hit: false,
                                exact_hit: false,
                                alias_hit: false,
                                gram_hits: 0,
                            })
                            .gram_hits += 1;
                    }
                }
            }
        }

        let mut ranked = signals.into_iter().collect::<Vec<_>>();
        ranked.sort_by(|left, right| compare_retrieval_hits(left, right, lexicon));
        ranked.truncate(self.config.beam_retrieval_shortlist);
        ranked
    }

    fn score_span_candidate(
        &self,
        start: usize,
        end: usize,
        span: &str,
        entry: &RankedLexiconEntry,
        search_keys: &[String],
        signals: &RetrievalSignals,
    ) -> Option<SpanCandidate> {
        let mut best_edit = 0.0f64;
        let mut best_ngram = 0.0f64;
        for query in search_keys {
            for form in entry.score_forms() {
                best_edit = best_edit.max(weighted_similarity(query, form));
                best_ngram = best_ngram.max(dice_score(query, form));
            }
        }

        if best_edit < CHUNK_EDIT_FLOOR && !signals.exact_hit && !signals.alias_hit {
            return None;
        }

        let prefix_bonus = onset_bonus(span, &entry.normalized_key);
        let exact_bonus = if signals.exact_hit { 3_200 } else { 0 };
        let alias_bonus = if signals.alias_hit { 2_300 } else { 0 };
        let edit_score = (best_edit * 4_600.0).round() as i32;
        let ngram_score = (best_ngram * 2_200.0).round() as i32;
        let frequency_score = frequency_prior(entry.frequency);
        let source_score = source_rank_prior(entry.source_rank);
        let span_len = end.saturating_sub(start) as i32;
        let span_bonus = span_len * 220;
        let long_span_bonus = if span_len >= 5 && (signals.exact_hit || signals.alias_hit || best_edit >= 0.72) {
            900
        } else {
            0
        };
        let score = exact_bonus
            + alias_bonus
            + edit_score
            + ngram_score
            + prefix_bonus
            + frequency_score
            + source_score
            + span_bonus
            + long_span_bonus;

        Some(SpanCandidate {
            start,
            end,
            input: span.to_owned(),
            output: entry.target.clone(),
            recovered_roman: entry.canonical_roman.clone(),
            source_rank: entry.source_rank,
            first_tag: entry.first_tag.clone(),
            last_tag: entry.last_tag.clone(),
            score,
            score_bps: score_to_bps(score),
            edit_similarity: best_edit,
        })
    }

    fn extend_beam(&self, mut beam: BeamItem, candidate: &SpanCandidate, request: &DecodeRequest<'_>) -> BeamItem {
        beam.position = candidate.end;
        beam.scores.chunk += candidate.score;
        beam.scores.segmentation += segmentation_delta(candidate.input.chars().count(), candidate.edit_similarity);
        beam.scores.segmentation += composer_alignment_delta(request.composer, candidate.start, candidate.end);
        beam.scores.lm += lm_delta(self.data.ranked(), beam.words.last(), &candidate.output);
        beam.scores.pos += pos_delta(
            self.data.ranked(),
            beam.last_tags.last().and_then(|tag| tag.as_deref()),
            candidate.first_tag.as_deref(),
        );
        beam.scores.history += incremental_history_score(request.history, &candidate.output);
        beam.recovered_romans.push(candidate.recovered_roman.clone());
        beam.words.push(candidate.output.clone());
        beam.last_tags.push(candidate.last_tag.clone());
        beam.spans.push(candidate.clone());
        beam
    }

    fn has_anchor_chunks(&self, request: &DecodeRequest<'_>) -> bool {
        request
            .composer
            .all_wfst_phrase_chunks()
            .iter()
            .flatten()
            .any(|chunk| chunk.end.saturating_sub(chunk.start) >= MIN_SPAN_LEN)
    }

    fn over_budget(&self, started_at: super::DecodeTimer) -> bool {
        self.config.interactive_mode
            && elapsed_decode_us(started_at) > self.config.wfst_max_latency_ms.saturating_mul(1_000)
    }

    fn preferred_anchor_ranges(&self, request: &DecodeRequest<'_>, total_len: usize) -> Option<Vec<(usize, usize)>> {
        request.composer.all_wfst_phrase_chunks().iter().find_map(|chunks| {
            if chunks.len() < 2 {
                return None;
            }

            let ranges = chunks
                .iter()
                .filter_map(|chunk| {
                    let len = chunk.end.saturating_sub(chunk.start);
                    (len >= MIN_SPAN_LEN).then_some((chunk.start, chunk.end))
                })
                .collect::<Vec<_>>();

            if ranges.len() < 2 {
                return None;
            }

            let explicit = chunks
                .iter()
                .any(|chunk| chunk.kind == crate::composer::ComposerChunkKind::Explicit);
            let all_strong = ranges.iter().all(|(start, end)| end.saturating_sub(*start) >= 4);
            let all_exact = chunks
                .iter()
                .all(|chunk| chunk.kind == crate::composer::ComposerChunkKind::Exact);
            if !explicit && ranges.len() > 2 && !all_strong && !all_exact {
                return None;
            }

            let covered = ranges.last().map(|(_, end)| *end).unwrap_or_default() == total_len
                && ranges.first().map(|(start, _)| *start).unwrap_or_default() == 0;
            covered.then_some(ranges)
        })
    }

    fn preferred_lattice_ends(&self, request: &DecodeRequest<'_>, total_len: usize) -> HashMap<usize, Vec<usize>> {
        if !self.config.interactive_mode {
            return HashMap::new();
        }

        let mut by_start = HashMap::<usize, HashSet<usize>>::new();
        for path in request.composer.all_wfst_phrase_chunks() {
            for chunk in path {
                let len = chunk.end.saturating_sub(chunk.start);
                if len < MIN_SPAN_LEN {
                    continue;
                }
                by_start.entry(chunk.start).or_default().insert(chunk.end);
                if chunk.kind == crate::composer::ComposerChunkKind::Hint {
                    if chunk.end > chunk.start + MIN_SPAN_LEN {
                        by_start.entry(chunk.start).or_default().insert(chunk.end - 1);
                    }
                    if chunk.end < total_len {
                        by_start.entry(chunk.start).or_default().insert(chunk.end + 1);
                    }
                }
            }
        }

        by_start
            .into_iter()
            .map(|(start, ends)| {
                let mut ends = ends.into_iter().collect::<Vec<_>>();
                ends.sort_unstable();
                (start, ends)
            })
            .collect()
    }
}

impl Decoder for WfstDecoder {
    fn name(&self) -> &'static str {
        "wfst"
    }

    fn decode(&self, request: &DecodeRequest<'_>) -> DecodeResult {
        let started_at = start_decode_timer();
        if request.composer.normalized.is_empty() {
            return DecodeResult::failed(self.name(), DecodeFailure::EmptyResult, elapsed_decode_us(started_at));
        }

        let finals = self.decode_with_beam(request, started_at);
        if finals.is_empty() && self.over_budget(started_at) {
            return DecodeResult::failed(self.name(), DecodeFailure::Timeout, elapsed_decode_us(started_at));
        }
        if finals.is_empty() {
            return DecodeResult::failed(self.name(), DecodeFailure::EmptyResult, elapsed_decode_us(started_at));
        }

        let mut best_total = finals
            .iter()
            .map(|item| item.total_score(&self.config))
            .max()
            .unwrap_or_default();
        if best_total <= 0 {
            best_total = 1;
        }

        let candidates = finals
            .into_iter()
            .map(|item| beam_item_to_candidate(item, best_total, &self.config))
            .fold(Vec::<DecodeCandidate>::new(), |mut output, candidate| {
                if !output.iter().any(|current| current.text == candidate.text) {
                    output.push(candidate);
                }
                output
            });
        DecodeResult::success(self.name(), candidates, elapsed_decode_us(started_at))
    }
}

fn beam_item_to_candidate(item: BeamItem, best_total: i32, config: &DecoderConfig) -> DecodeCandidate {
    let total = item.total_score(config);
    let confidence =
        (item.spans.iter().map(|span| span.edit_similarity).sum::<f64>() / item.spans.len().max(1) as f64) * 10_000.0;
    let relative = ((total as f64 / best_total.max(1) as f64) * 10_000.0).round() as i32;

    DecodeCandidate {
        text: item.final_text(),
        score_bps: Some(relative.clamp(0, 10_000) as u16),
        confidence_bps: Some(confidence.round().clamp(0.0, 10_000.0) as u16),
        segments: item
            .spans
            .into_iter()
            .map(|span| DecodeSegment {
                input: span.input,
                output: span.output,
                weight_bps: span.score_bps,
            })
            .collect(),
    }
}

fn prune_beam(items: &mut Vec<BeamItem>, width: usize, config: &DecoderConfig) {
    items.sort_by(|left, right| compare_beam_items(left, right, config));
    items.truncate(width.max(1));
}

fn compare_beam_items(left: &BeamItem, right: &BeamItem, config: &DecoderConfig) -> std::cmp::Ordering {
    right
        .total_score(config)
        .cmp(&left.total_score(config))
        .then_with(|| left.spans.len().cmp(&right.spans.len()))
        .then_with(|| right.position.cmp(&left.position))
        .then_with(|| {
            right
                .words
                .iter()
                .map(|word| word.chars().count())
                .sum::<usize>()
                .cmp(&left.words.iter().map(|word| word.chars().count()).sum::<usize>())
        })
        .then_with(|| left.final_text().cmp(&right.final_text()))
        .then_with(|| left.recovered_romans.cmp(&right.recovered_romans))
}

fn compare_span_candidates(left: &SpanCandidate, right: &SpanCandidate) -> std::cmp::Ordering {
    right
        .score
        .cmp(&left.score)
        .then_with(|| {
            right
                .end
                .saturating_sub(right.start)
                .cmp(&left.end.saturating_sub(left.start))
        })
        .then_with(|| right.edit_similarity.total_cmp(&left.edit_similarity))
        .then_with(|| left.source_rank.cmp(&right.source_rank))
        .then_with(|| left.recovered_roman.cmp(&right.recovered_roman))
        .then_with(|| left.output.cmp(&right.output))
}

fn compare_retrieval_hits(
    left: &(usize, RetrievalSignals),
    right: &(usize, RetrievalSignals),
    lexicon: &RankedLexicon,
) -> std::cmp::Ordering {
    let left_entry = &lexicon.entries[left.0];
    let right_entry = &lexicon.entries[right.0];
    let left_signal = &left.1;
    let right_signal = &right.1;

    right_signal
        .raw_exact_hit
        .cmp(&left_signal.raw_exact_hit)
        .then_with(|| right_signal.exact_hit.cmp(&left_signal.exact_hit))
        .then_with(|| right_signal.alias_hit.cmp(&left_signal.alias_hit))
        .then_with(|| right_signal.gram_hits.cmp(&left_signal.gram_hits))
        .then_with(|| right_entry.frequency.cmp(&left_entry.frequency))
        .then_with(|| left_entry.source_rank.cmp(&right_entry.source_rank))
        .then_with(|| left_entry.canonical_roman.cmp(&right_entry.canonical_roman))
        .then_with(|| left_entry.target.cmp(&right_entry.target))
}

fn frequency_prior(frequency: u32) -> i32 {
    (((frequency + 1) as f64).ln() * 120.0).round() as i32
}

fn source_rank_prior(source_rank: usize) -> i32 {
    1_800i32.saturating_sub((source_rank / 2) as i32)
}

fn score_to_bps(score: i32) -> u16 {
    score.clamp(0, 10_000) as u16
}

fn segmentation_delta(span_len: usize, edit_similarity: f64) -> i32 {
    let length_bonus = (span_len as i32) * 280;
    let chunk_penalty = 2_200;
    let micro_penalty = match span_len {
        0..=2 => 1_800,
        3 => 1_000,
        4 => 500,
        _ => -400,
    };
    let quality_bonus = if span_len >= 5 && edit_similarity >= 0.62 {
        800
    } else {
        0
    };
    length_bonus + quality_bonus - chunk_penalty - micro_penalty
}

fn lm_delta(lexicon: &RankedLexicon, previous: Option<&String>, word: &str) -> i32 {
    let corpus_surface = lexicon.corpus_surface_unigrams.get(word).copied().unwrap_or(0);
    let corpus_word = lexicon.corpus_word_unigrams.get(word).copied().unwrap_or(0);
    let lexicon_unigram = lexicon.word_unigrams.get(word).copied().unwrap_or(0);
    let unigram_score = if corpus_surface > 0 {
        (((corpus_surface + 1) as f64).ln() * 260.0).round() as i32
    } else if corpus_word > 0 {
        (((corpus_word + 1) as f64).ln() * 240.0).round() as i32
    } else {
        (((lexicon_unigram + 1) as f64).ln() * 180.0).round() as i32
    };
    let bigram_score = previous
        .map(|prev| {
            let corpus_bigram = lexicon
                .corpus_word_bigrams
                .get(&(prev.clone(), word.to_owned()))
                .copied()
                .unwrap_or(0);
            if corpus_bigram > 0 {
                return (((corpus_bigram + 1) as f64).ln() * 320.0).round() as i32;
            }

            let lexicon_bigram = lexicon
                .word_bigrams
                .get(&(prev.clone(), word.to_owned()))
                .copied()
                .unwrap_or(0);
            if lexicon_bigram > 0 {
                return (((lexicon_bigram + 1) as f64).ln() * 260.0).round() as i32;
            }

            if lexicon.corpus_surface_unigrams.contains_key(prev) && lexicon.corpus_surface_unigrams.contains_key(word)
            {
                -40
            } else {
                -90
            }
        })
        .unwrap_or(0);
    unigram_score + bigram_score
}

fn pos_delta(lexicon: &RankedLexicon, previous: Option<&str>, current: Option<&str>) -> i32 {
    let (Some(previous), Some(current)) = (previous, current) else {
        return 0;
    };

    let transition = lexicon
        .tag_bigrams
        .get(&(previous.to_owned(), current.to_owned()))
        .copied()
        .unwrap_or(0);
    if transition > 0 {
        return (((transition + 1) as f64).ln() * 28.0).round() as i32;
    }

    let previous_seen = lexicon.tag_unigrams.contains_key(previous);
    let current_seen = lexicon.tag_unigrams.contains_key(current);
    if previous_seen && current_seen {
        -18
    } else {
        0
    }
}

fn composer_alignment_delta(composer: &crate::composer::ComposerAnalysis, start: usize, end: usize) -> i32 {
    if composer
        .chunks
        .iter()
        .any(|chunk| chunk.end.saturating_sub(chunk.start) < 3)
    {
        return 0;
    }

    let mut score = 0i32;
    for chunk in &composer.chunks {
        let chunk_len = chunk.end.saturating_sub(chunk.start);
        if chunk_len < 4 {
            continue;
        }
        if start == chunk.start {
            score += 260;
        } else if chunk.start < start && start < chunk.end {
            score -= 650;
        }

        if end == chunk.end {
            score += 260;
        } else if chunk.start < end && end < chunk.end {
            score -= 650;
        }

        if start < chunk.start && chunk.start < end {
            score -= 950;
        }
        if start < chunk.end && chunk.end < end {
            score -= 950;
        }
    }
    score
}

fn incremental_history_score(history: &HashMap<String, usize>, word: &str) -> i32 {
    history.get(word).copied().unwrap_or(0).min(8) as i32 * 80
}

fn final_history_score(history: &HashMap<String, usize>, phrase: &str) -> i32 {
    history.get(phrase).copied().unwrap_or(0).min(12) as i32 * 140
}

fn onset_bonus(left: &str, right: &str) -> i32 {
    let left_prefix = onset_key(left);
    let right_prefix = onset_key(right);
    if left_prefix.is_empty() || right_prefix.is_empty() {
        return 0;
    }
    if left_prefix == right_prefix {
        500
    } else if left_prefix.starts_with(&right_prefix) || right_prefix.starts_with(&left_prefix) {
        220
    } else {
        0
    }
}

fn onset_key(input: &str) -> String {
    let mut output = String::new();
    for ch in input.chars() {
        if matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u' | 'y') {
            break;
        }
        output.push(ch);
        if output.len() >= 3 {
            break;
        }
    }
    output
}

fn dice_score(left: &str, right: &str) -> f64 {
    let left_grams = char_ngrams(left, 2);
    let right_grams = char_ngrams(right, 2);
    if left_grams.is_empty() || right_grams.is_empty() {
        return if left == right { 1.0 } else { 0.0 };
    }
    let left_len = left_grams.len();
    let right_len = right_grams.len();

    let mut right_counts = HashMap::<String, usize>::new();
    for gram in right_grams {
        *right_counts.entry(gram).or_default() += 1;
    }

    let mut overlap = 0usize;
    for gram in left_grams.iter() {
        if let Some(count) = right_counts.get_mut(gram) {
            if *count > 0 {
                *count -= 1;
                overlap += 1;
            }
        }
    }

    (2 * overlap) as f64 / (left_len + right_len) as f64
}

fn weighted_similarity(left: &str, right: &str) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }

    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut prev = (0..=left_chars.len()).map(|value| value as f64).collect::<Vec<_>>();
    let mut curr = vec![0.0f64; left_chars.len() + 1];

    for (row, &right_char) in right_chars.iter().enumerate() {
        curr[0] = (row + 1) as f64;
        for (col, &left_char) in left_chars.iter().enumerate() {
            let substitution = prev[col] + substitution_cost(left_char, right_char);
            let deletion = prev[col + 1] + deletion_cost(left_char, left_chars.get(col.wrapping_sub(1)).copied());
            let insertion = curr[col] + insertion_cost(right_char, right_chars.get(row.wrapping_sub(1)).copied());
            curr[col + 1] = substitution.min(deletion).min(insertion);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    let distance = prev[left_chars.len()];
    let denominator = left_chars.len().max(right_chars.len()).max(1) as f64;
    (1.0 - distance / denominator).clamp(0.0, 1.0)
}

fn substitution_cost(left: char, right: char) -> f64 {
    if left == right {
        return 0.0;
    }
    if both_vowels(left, right) {
        return 0.32;
    }
    if is_soft_final(left) && is_soft_final(right) {
        return 0.28;
    }
    if are_soft_confusions(left, right) {
        return 0.24;
    }
    1.0
}

fn insertion_cost(current: char, previous: Option<char>) -> f64 {
    if previous == Some(current) {
        return 0.12;
    }
    if is_vowel(current) {
        return 0.58;
    }
    0.88
}

fn deletion_cost(current: char, previous: Option<char>) -> f64 {
    insertion_cost(current, previous)
}

fn are_soft_confusions(left: char, right: char) -> bool {
    matches!(
        (left, right),
        ('k', 'g')
            | ('g', 'k')
            | ('t', 'd')
            | ('d', 't')
            | ('p', 'b')
            | ('b', 'p')
            | ('s', 'h')
            | ('h', 's')
            | ('s', 'r')
            | ('r', 's')
            | ('r', 'h')
            | ('h', 'r')
            | ('n', 'm')
            | ('m', 'n')
    )
}

fn both_vowels(left: char, right: char) -> bool {
    is_vowel(left) && is_vowel(right)
}

fn is_vowel(ch: char) -> bool {
    matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

fn is_soft_final(ch: char) -> bool {
    matches!(ch, 'h' | 's' | 'r')
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::{lm_delta, pos_delta, WfstDecoder};
    use crate::decoder::{DecoderConfig, DecoderMode};
    use crate::roman_lookup::{LegacyData, RankedLexicon, Transliterator};

    #[test]
    fn returns_candidates_for_exact_entries() {
        let transliterator =
            Transliterator::from_default_data_with_config(DecoderConfig::default().with_mode(DecoderMode::Wfst))
                .unwrap();

        assert_eq!(
            transliterator
                .suggest("jea", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("ជា")
        );
    }

    #[test]
    fn composes_exact_chunk_phrases_without_spaces() {
        let transliterator =
            Transliterator::from_default_data_with_config(DecoderConfig::default().with_mode(DecoderMode::Wfst))
                .unwrap();

        assert_eq!(
            transliterator
                .suggest("khnhomttov", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("ខ្ញុំទៅ")
        );
    }

    #[test]
    fn recovers_noisy_long_phrase_with_beam_search() {
        let transliterator =
            Transliterator::from_default_data_with_config(DecoderConfig::default().with_mode(DecoderMode::Wfst))
                .unwrap();

        assert_eq!(
            transliterator
                .suggest("knhhomttovsalarien", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("ខ្ញុំទៅសាលារៀន")
        );
    }

    #[test]
    fn prefers_long_meaningful_span_over_fragmentation() {
        let transliterator =
            Transliterator::from_default_data_with_config(DecoderConfig::default().with_mode(DecoderMode::Wfst))
                .unwrap();

        assert_eq!(
            transliterator
                .suggest("saensronors", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("សែនស្រណោះ")
        );
    }

    #[test]
    fn single_span_reranker_prefers_sronaoh_variant() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let data = Arc::new(LegacyData::from_entries(transliterator.entries().to_vec()));
        let decoder = WfstDecoder::new(data, DecoderConfig::default().with_mode(DecoderMode::Wfst));

        let candidates = decoder.rerank_span_candidates(0, 7, "sronors");
        assert_eq!(
            candidates.first().map(|candidate| candidate.output.as_str()),
            Some("ស្រណោះ")
        );
    }

    #[test]
    fn prefers_exact_single_span_match_for_sronos() {
        let transliterator =
            Transliterator::from_default_data_with_config(DecoderConfig::default().with_mode(DecoderMode::Wfst))
                .unwrap();

        assert_eq!(
            transliterator
                .suggest("sronos", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("ស្រណោះ")
        );
    }

    #[test]
    fn lm_delta_prefers_corpus_surface_counts_for_joined_outputs() {
        let mut lexicon = RankedLexicon::default();
        lexicon.corpus_surface_unigrams.insert(String::from("ខ្ញុំទៅ"), 3);
        lexicon.word_unigrams.insert(String::from("ខ្ញុំទៅ"), 1);

        assert!(lm_delta(&lexicon, None, "ខ្ញុំទៅ") > 0);
    }

    #[test]
    fn pos_delta_prefers_seen_tag_transitions() {
        let mut lexicon = RankedLexicon::default();
        lexicon.tag_unigrams.insert(String::from("PRO"), 10);
        lexicon.tag_unigrams.insert(String::from("VB"), 12);
        lexicon.tag_unigrams.insert(String::from("JJ"), 4);
        lexicon
            .tag_bigrams
            .insert((String::from("PRO"), String::from("VB")), 25);

        assert!(pos_delta(&lexicon, Some("PRO"), Some("VB")) > 0);
        assert!(pos_delta(&lexicon, Some("PRO"), Some("JJ")) < 0);
        assert_eq!(pos_delta(&lexicon, None, Some("VB")), 0);
    }
}
