use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::roman_lookup::{char_ngrams, roman_search_variants, LegacyData, RankedEntryView};

use super::{
    elapsed_decode_us, provider_for_mode, start_decode_timer, DecodeCandidate, DecodeFailure, DecodeRequest,
    DecodeResult, DecodeSegment, Decoder, DecoderConfig, SpanProposal, SpanProposalProvider, SpanProposalRequest,
};

/// Test-only counter of how many times the Weighted Span decoder has run. Lets tests assert the
/// per-keystroke decode budget (e.g. one recompute must not run Weighted Span twice). Compiled in
/// all builds — one relaxed atomic increment per decode is negligible — but only read from tests.
pub static WEIGHTED_SPAN_DECODE_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Reset the decode counter to zero. Test-only helper.
pub fn reset_weighted_span_decode_calls() {
    WEIGHTED_SPAN_DECODE_CALLS.store(0, std::sync::atomic::Ordering::Relaxed);
}

/// Read the decode counter. Test-only helper.
pub fn weighted_span_decode_calls() -> usize {
    WEIGHTED_SPAN_DECODE_CALLS.load(std::sync::atomic::Ordering::Relaxed)
}

const MIN_SPAN_LEN: usize = 3;
const MIN_EXACT_ANCHOR_LEN: usize = 2;
const CHUNK_EDIT_FLOOR: f64 = 0.46;
const RARE_ENTRY_FUZZY_FLOOR: f64 = 0.65;
const RARE_ENTRY_FREQUENCY_MAX: u32 = 1;
const SEGMENT_LIMIT_PER_START: usize = 12;

// Weighted span decoder.
//
// This is not a compiled WFST implementation. It builds a lattice of candidate
// roman-input spans from lexicon retrieval signals, then uses beam search to
// choose the best sequence of output chunks.
//
// Scores are kept as separate components so shadow/hybrid modes can explain why
// a candidate won without exposing decoder internals to platform adapters.

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
    first_tag: Option<String>,
    last_tag: Option<String>,
    score: i32,
    score_bps: u16,
    edit_similarity: f64,
    /// True when this span came from the model span-proposal provider (vs lexicon retrieval),
    /// so downstream candidates can be marked as model-assisted.
    from_model: bool,
    lexicon_verified: bool,
}

#[derive(Clone, Debug, Default)]
struct ScoreBreakdown {
    chunk: i32,
    segmentation: i32,
    context: i32,
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
            + self.context * config.context_score_weight
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

const WHOLE_DICTIONARY_TARGET_MIN_SPAN_LEN: usize = 5;
const WHOLE_DICTIONARY_TARGET_MIN_EDIT: f64 = 0.72;

pub(crate) struct WeightedSpanDecoder {
    span_proposer: Option<std::sync::Arc<dyn SpanProposalProvider>>,
    data: Arc<LegacyData>,
    config: DecoderConfig,
}

impl WeightedSpanDecoder {
    pub(crate) fn new(data: Arc<LegacyData>, config: DecoderConfig) -> Self {
        let span_proposer = provider_for_mode(config.span_proposal_mode);
        Self {
            data,
            config,
            span_proposer,
        }
    }

    fn decode_with_beam(&self, request: &DecodeRequest<'_>, started_at: super::DecodeTimer) -> Vec<BeamItem> {
        let chars = request.composer.normalized.chars().collect::<Vec<_>>();
        if chars.is_empty() {
            return Vec::new();
        }
        let exact_full_span = self.decode_exact_full_span(request, &chars, started_at);

        if self.config.interactive_mode && self.span_proposer.is_some() {
            let proposed = self.decode_with_span_proposals(request, &chars, started_at);
            if !proposed.is_empty() {
                let mut finals = proposed;
                finals.extend(self.decode_whole_dictionary_span(request, &chars, started_at));
                finals.extend(exact_full_span);
                // A model proposal is an additional hypothesis, not permission to discard
                // the Standard decoder's trusted exact-chunk phrase. Keep that anchored
                // baseline in the same ranking competition.
                finals.extend(self.decode_with_anchors(request, &chars, started_at));
                finals.sort_by(|left, right| compare_beam_items(left, right, &self.config));
                finals.truncate(self.config.max_candidates.max(self.config.beam_width));
                return finals;
            }
        }

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
        if !self.data.has_ranked_exact_key(span.as_str()) {
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
            beam.scores.segmentation += exact_anchor_path_bonus(request);
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
        let short_exact_ends = self.short_exact_ends_by_start(request);
        for start in 0..chars.len() {
            if self.over_budget(started_at) {
                return None;
            }
            let remaining = chars.len().saturating_sub(start);
            let preferred = preferred_ends.get(&start);
            let short_exact = short_exact_ends.get(&start);
            if remaining < MIN_SPAN_LEN && preferred.is_none() && short_exact.is_none() {
                continue;
            }
            let mut best_by_signature = HashMap::<(usize, String), SpanCandidate>::new();
            let mut lengths: Vec<usize> = if let Some(ends) = preferred {
                ends.iter()
                    .copied()
                    .filter(|end| *end > start)
                    .map(|end| end - start)
                    .collect()
            } else {
                let max_len = self.config.beam_max_span_len.min(remaining);
                let mut lengths = (MIN_SPAN_LEN..=max_len).rev().collect::<Vec<_>>();
                lengths.truncate(self.config.beam_lengths_per_start.max(1));
                lengths
            };
            if let Some(ends) = short_exact {
                for end in ends {
                    if *end > start {
                        let len = end - start;
                        if !lengths.contains(&len) {
                            lengths.push(len);
                        }
                    }
                }
            }

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
            let entry = self.data.ranked_entry(entry_index);
            if let Some(candidate) = self.score_span_candidate(start, end, span, &entry, &search_keys, &signals) {
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
        let data = self.data.as_ref();
        let raw_exact_ids = self.data.ranked_exact_entry_ids(raw_key);
        if !raw_exact_ids.is_empty() {
            let mut ranked = raw_exact_ids
                .into_iter()
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
            ranked.sort_by(|left, right| compare_retrieval_hits(left, right, data));
            ranked.truncate(self.config.beam_retrieval_shortlist.max(1));
            return ranked;
        }
        let mut exact_signals = HashMap::<usize, RetrievalSignals>::new();
        let mut alias_only = HashMap::<usize, RetrievalSignals>::new();

        for key in search_keys {
            for entry_index in self.data.ranked_exact_entry_ids(key) {
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
            for entry_index in self.data.ranked_alias_entry_ids(key) {
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

        if self.config.interactive_mode && !exact_signals.is_empty() {
            let mut ranked = exact_signals.into_iter().collect::<Vec<_>>();
            ranked.sort_by(|left, right| compare_retrieval_hits(left, right, data));
            ranked.truncate(self.config.beam_retrieval_shortlist.max(1));
            return ranked;
        }

        if self.config.interactive_mode && raw_key.chars().count() >= 4 && !alias_only.is_empty() {
            let mut ranked = alias_only.into_iter().collect::<Vec<_>>();
            ranked.sort_by(|left, right| compare_retrieval_hits(left, right, data));
            ranked.truncate(self.config.beam_retrieval_shortlist.max(1));
            return ranked;
        }

        let mut signals = HashMap::<usize, RetrievalSignals>::new();

        for key in search_keys {
            for entry_index in self.data.ranked_exact_entry_ids(key) {
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
            for entry_index in self.data.ranked_alias_entry_ids(key) {
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
            for gram in char_ngrams(key, 2) {
                for entry_index in self.data.ranked_gram_entry_ids(&gram) {
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

        let mut ranked = signals.into_iter().collect::<Vec<_>>();
        ranked.sort_by(|left, right| compare_retrieval_hits(left, right, data));
        ranked.truncate(self.config.beam_retrieval_shortlist);
        ranked
    }

    fn score_span_candidate(
        &self,
        start: usize,
        end: usize,
        span: &str,
        entry: &RankedEntryView,
        search_keys: &[String],
        signals: &RetrievalSignals,
    ) -> Option<SpanCandidate> {
        let mut best_edit = 0.0f64;
        let mut best_ngram = 0.0f64;
        let forms = entry.score_forms();
        for query in search_keys {
            for &form in &forms {
                best_edit = best_edit.max(weighted_similarity(query, form));
                best_ngram = best_ngram.max(dice_score(query, form));
            }
        }

        if best_edit < CHUNK_EDIT_FLOOR && !signals.exact_hit && !signals.alias_hit {
            return None;
        }

        if entry.frequency() <= RARE_ENTRY_FREQUENCY_MAX
            && best_edit < RARE_ENTRY_FUZZY_FLOOR
            && !signals.exact_hit
            && !signals.alias_hit
        {
            return None;
        }

        let prefix_bonus = onset_bonus(span, entry.normalized_key());
        let exact_bonus = if signals.exact_hit { 3_200 } else { 0 };
        let alias_bonus = if signals.alias_hit { 2_300 } else { 0 };
        let edit_score = (best_edit * 4_600.0).round() as i32;
        let ngram_score = (best_ngram * 2_200.0).round() as i32;
        let frequency_score = frequency_prior(entry.frequency());
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
            + span_bonus
            + long_span_bonus;

        Some(SpanCandidate {
            start,
            end,
            input: span.to_owned(),
            output: entry.target().to_owned(),
            recovered_roman: entry.canonical_roman().to_owned(),
            first_tag: entry.first_tag().map(str::to_owned),
            last_tag: entry.last_tag().map(str::to_owned),
            score,
            score_bps: score_to_bps(score),
            edit_similarity: best_edit,
            from_model: false,
            lexicon_verified: true,
        })
    }

    fn extend_beam(&self, mut beam: BeamItem, candidate: &SpanCandidate, request: &DecodeRequest<'_>) -> BeamItem {
        beam.position = candidate.end;
        beam.scores.chunk += candidate.score;
        beam.scores.segmentation += segmentation_delta(candidate.input.chars().count(), candidate.edit_similarity);
        beam.scores.segmentation += composer_alignment_delta(request.composer, candidate.start, candidate.end);
        beam.scores.context += context_delta(self.data.as_ref(), beam.words.last(), &candidate.output);
        beam.scores.pos += pos_delta(
            self.data.as_ref(),
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
            .all_weighted_span_phrase_chunks()
            .iter()
            .flatten()
            .any(|chunk| chunk.end.saturating_sub(chunk.start) >= chunk_min_anchor_len(chunk))
    }

    fn over_budget(&self, started_at: super::DecodeTimer) -> bool {
        self.config.interactive_mode
            && elapsed_decode_us(started_at) > self.config.wfst_max_latency_ms.saturating_mul(1_000)
    }

    fn preferred_anchor_ranges(&self, request: &DecodeRequest<'_>, total_len: usize) -> Option<Vec<(usize, usize)>> {
        request
            .composer
            .all_weighted_span_phrase_chunks()
            .iter()
            .find_map(|chunks| {
                if chunks.len() < 2 {
                    return None;
                }

                let ranges = chunks
                    .iter()
                    .filter_map(|chunk| {
                        let len = chunk.end.saturating_sub(chunk.start);
                        let floor = chunk_min_anchor_len(chunk);
                        (len >= floor).then_some((chunk.start, chunk.end))
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
                let contiguous = ranges.windows(2).all(|pair| pair[0].1 == pair[1].0);
                (covered && contiguous).then_some(ranges)
            })
    }

    fn preferred_lattice_ends(&self, request: &DecodeRequest<'_>, total_len: usize) -> HashMap<usize, Vec<usize>> {
        if !self.config.interactive_mode {
            return HashMap::new();
        }

        let mut by_start = HashMap::<usize, HashSet<usize>>::new();
        for path in request.composer.all_weighted_span_phrase_chunks() {
            for chunk in path {
                let len = chunk.end.saturating_sub(chunk.start);
                if len < chunk_min_anchor_len(chunk) {
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

    fn short_exact_ends_by_start(&self, request: &DecodeRequest<'_>) -> HashMap<usize, Vec<usize>> {
        let mut by_start = HashMap::<usize, HashSet<usize>>::new();
        for chunk in &request.composer.chunks {
            if chunk.kind != crate::composer::ComposerChunkKind::Exact {
                continue;
            }
            let len = chunk.end.saturating_sub(chunk.start);
            if !(MIN_EXACT_ANCHOR_LEN..MIN_SPAN_LEN).contains(&len) {
                continue;
            }
            by_start.entry(chunk.start).or_default().insert(chunk.end);
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

    fn decode_whole_dictionary_span(
        &self,
        request: &DecodeRequest<'_>,
        chars: &[char],
        started_at: super::DecodeTimer,
    ) -> Vec<BeamItem> {
        if self.over_budget(started_at) {
            return Vec::new();
        }
        let span = chars.iter().collect::<String>();
        self.rerank_span_candidates_with_context(0, chars.len(), &span, &request.composer.normalized)
            .into_iter()
            .filter(|candidate| self.is_strong_whole_dictionary_target(candidate, chars.len()))
            .map(|candidate| {
                let mut beam = self.extend_beam(BeamItem::default(), &candidate, request);
                beam.scores.segmentation += 26_000;
                beam.scores.history += final_history_score(request.history, &beam.final_text());
                beam
            })
            .collect()
    }

    fn is_strong_whole_dictionary_target(&self, candidate: &SpanCandidate, full_len: usize) -> bool {
        candidate.start == 0
            && candidate.end == full_len
            && full_len >= WHOLE_DICTIONARY_TARGET_MIN_SPAN_LEN
            && candidate.edit_similarity >= WHOLE_DICTIONARY_TARGET_MIN_EDIT
            && candidate.output.split_whitespace().count() == 1
            && self.data.target_frequency_for(&candidate.output) > RARE_ENTRY_FREQUENCY_MAX
    }

    fn apply_whole_dictionary_target_bonus(&self, candidate: &mut SpanCandidate, full_input: &str) {
        let full_len = full_input.chars().count();
        let span_len = candidate.end.saturating_sub(candidate.start);
        if candidate.start != 0
            || candidate.end != full_len
            || span_len < WHOLE_DICTIONARY_TARGET_MIN_SPAN_LEN
            || candidate.edit_similarity < WHOLE_DICTIONARY_TARGET_MIN_EDIT
            || candidate.output.split_whitespace().count() != 1
        {
            return;
        }

        let frequency = self.data.target_frequency_for(&candidate.output);
        if frequency <= RARE_ENTRY_FREQUENCY_MAX {
            return;
        }

        let frequency_bonus = (((frequency + 1) as f64).ln() * 420.0).round() as i32;
        candidate.score += 2_800 + frequency_bonus.min(2_400);
        candidate.score_bps = score_to_bps(candidate.score);
    }

    fn decode_with_span_proposals(
        &self,
        request: &DecodeRequest<'_>,
        chars: &[char],
        started_at: super::DecodeTimer,
    ) -> Vec<BeamItem> {
        let Some(provider) = &self.span_proposer else {
            return Vec::new();
        };

        let mut beams = vec![Vec::<BeamItem>::new(); chars.len() + 1];
        beams[0].push(BeamItem::default());

        for start in 0..chars.len() {
            if self.over_budget(started_at) {
                break;
            }
            if beams[start].is_empty() {
                continue;
            }

            let current = beams[start].clone();
            for end in provider.candidate_ends(&request.composer.normalized, start, chars) {
                if end <= start || end > chars.len() {
                    continue;
                }
                let span = chars[start..end].iter().collect::<String>();
                let candidates = self.proposed_span_candidates(start, end, &span, &request.composer.normalized);
                for beam in &current {
                    for candidate in &candidates {
                        let next = self.extend_beam(beam.clone(), candidate, request);
                        let bucket = &mut beams[candidate.end];
                        bucket.push(next);
                        prune_beam(bucket, self.config.beam_width, &self.config);
                    }
                }
            }
        }

        let mut finals = beams[chars.len()]
            .iter()
            .cloned()
            .map(|mut item| {
                item.scores.segmentation += 18_000;
                item.scores.history += final_history_score(request.history, &item.final_text());
                item
            })
            .collect::<Vec<_>>();
        finals.sort_by(|left, right| compare_beam_items(left, right, &self.config));
        finals.truncate(self.config.max_candidates.max(self.config.beam_width));
        finals
    }

    fn rerank_span_candidates_with_context(
        &self,
        start: usize,
        end: usize,
        span: &str,
        full_input: &str,
    ) -> Vec<SpanCandidate> {
        let search_keys = roman_search_variants(span);
        if search_keys.is_empty() {
            return Vec::new();
        }

        let raw_key = search_keys.first().map(String::as_str).unwrap_or(span);
        let retrieved = self.retrieve_candidates(&search_keys, raw_key);
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
            let entry = self.data.ranked_entry(entry_index);
            if let Some(mut candidate) = self.score_span_candidate(start, end, span, &entry, &search_keys, &signals) {
                self.apply_whole_dictionary_target_bonus(&mut candidate, full_input);
                match best_by_output.get(candidate.output.as_str()) {
                    Some(current) if !compare_span_candidates(current, &candidate).is_gt() => {}
                    _ => {
                        best_by_output.insert(candidate.output.clone(), candidate);
                    }
                }
            }
        }
        for candidate in self.proposed_span_candidates(start, end, span, full_input) {
            match best_by_output.get(candidate.output.as_str()) {
                Some(current) if !compare_span_candidates(current, &candidate).is_gt() => {}
                _ => {
                    best_by_output.insert(candidate.output.clone(), candidate);
                }
            }
        }

        let mut ranked = best_by_output.into_values().collect::<Vec<_>>();
        ranked.sort_by(|left, right| compare_span_candidates(left, right));
        ranked.truncate(self.config.beam_candidates_per_span);
        ranked
    }

    fn proposed_span_candidates(&self, start: usize, end: usize, span: &str, full_input: &str) -> Vec<SpanCandidate> {
        let Some(provider) = &self.span_proposer else {
            return Vec::new();
        };
        provider
            .propose(&SpanProposalRequest {
                full_input,
                span,
                start,
                end,
            })
            .into_iter()
            .filter_map(|proposal| self.span_candidate_from_proposal(start, end, span, proposal))
            .collect()
    }

    fn span_candidate_from_proposal(
        &self,
        start: usize,
        end: usize,
        span: &str,
        proposal: SpanProposal,
    ) -> Option<SpanCandidate> {
        if proposal.output.is_empty() {
            return None;
        }

        let lexicon_verified = self.data.has_target(&proposal.output);
        let confidence = proposal.confidence_bps.min(10_000);
        let span_len = end.saturating_sub(start) as i32;
        let score = 3_200
            + (i32::from(confidence) * 5_200 / 10_000)
            + frequency_prior(self.data.target_frequency_for(&proposal.output))
            + span_len * 220;

        Some(SpanCandidate {
            start,
            end,
            input: span.to_owned(),
            output: proposal.output,
            recovered_roman: if proposal.recovered_roman.is_empty() {
                span.to_owned()
            } else {
                proposal.recovered_roman
            },
            first_tag: None,
            last_tag: None,
            score,
            score_bps: score_to_bps(score),
            edit_similarity: f64::from(confidence) / 10_000.0,
            from_model: true,
            lexicon_verified,
        })
    }
}

impl Decoder for WeightedSpanDecoder {
    fn name(&self) -> &'static str {
        "weighted_span"
    }

    fn decode(&self, request: &DecodeRequest<'_>) -> DecodeResult {
        WEIGHTED_SPAN_DECODE_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
                if let Some(current) = output.iter_mut().find(|current| current.text == candidate.text) {
                    // Keep the best-ranked representation, but do not erase the fact that
                    // the model independently proposed the same text. Adapters use this
                    // provenance to show ✦ on AI alternatives.
                    current.from_model |= candidate.from_model;
                } else {
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
    let from_model = item.spans.iter().any(|span| span.from_model);
    let lexicon_verified = item.spans.iter().all(|span| span.lexicon_verified);

    DecodeCandidate {
        text: item.final_text(),
        score_bps: Some(relative.clamp(0, 10_000) as u16),
        confidence_bps: Some(confidence.round().clamp(0.0, 10_000.0) as u16),
        from_model,
        lexicon_verified,
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
        .then_with(|| left.recovered_roman.cmp(&right.recovered_roman))
        .then_with(|| left.output.cmp(&right.output))
}

fn compare_retrieval_hits(
    left: &(usize, RetrievalSignals),
    right: &(usize, RetrievalSignals),
    data: &LegacyData,
) -> std::cmp::Ordering {
    let left_entry = data.ranked_entry(left.0);
    let right_entry = data.ranked_entry(right.0);
    let left_signal = &left.1;
    let right_signal = &right.1;

    right_signal
        .raw_exact_hit
        .cmp(&left_signal.raw_exact_hit)
        .then_with(|| right_signal.exact_hit.cmp(&left_signal.exact_hit))
        .then_with(|| right_signal.alias_hit.cmp(&left_signal.alias_hit))
        .then_with(|| right_signal.gram_hits.cmp(&left_signal.gram_hits))
        .then_with(|| right_entry.frequency().cmp(&left_entry.frequency()))
        .then_with(|| left_entry.canonical_roman().cmp(right_entry.canonical_roman()))
        .then_with(|| left_entry.target().cmp(right_entry.target()))
        .then_with(|| left_entry.frequency_lang().cmp(right_entry.frequency_lang()))
}

fn frequency_prior(frequency: u32) -> i32 {
    (((frequency + 1) as f64).ln() * 120.0).round() as i32
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

fn context_delta(data: &LegacyData, previous: Option<&String>, word: &str) -> i32 {
    let corpus_surface = data.corpus_surface_unigram_count(word);
    let corpus_word = data.corpus_word_unigram_count(word);
    let lexicon_unigram = data.word_unigram_count(word);
    let unigram_score = if corpus_surface > 0 {
        (((corpus_surface + 1) as f64).ln() * 260.0).round() as i32
    } else if corpus_word > 0 {
        (((corpus_word + 1) as f64).ln() * 240.0).round() as i32
    } else {
        (((lexicon_unigram + 1) as f64).ln() * 180.0).round() as i32
    };
    let bigram_score = previous
        .map(|prev| {
            let corpus_bigram = data.corpus_word_bigram_count(prev, word);
            if corpus_bigram > 0 {
                return (((corpus_bigram + 1) as f64).ln() * 320.0).round() as i32;
            }

            let lexicon_bigram = data.word_bigram_count(prev, word);
            if lexicon_bigram > 0 {
                return (((lexicon_bigram + 1) as f64).ln() * 260.0).round() as i32;
            }

            if data.corpus_surface_unigram_count(prev) > 0 && data.corpus_surface_unigram_count(word) > 0 {
                -40
            } else {
                -90
            }
        })
        .unwrap_or(0);
    unigram_score + bigram_score
}

// Heuristic POS/context transition score from observed tag unigram and bigram counts.
fn pos_delta(data: &LegacyData, previous: Option<&str>, current: Option<&str>) -> i32 {
    let (Some(previous), Some(current)) = (previous, current) else {
        return 0;
    };

    let transition = data.tag_bigram_count(previous, current);
    if transition > 0 {
        return (((transition + 1) as f64).ln() * 28.0).round() as i32;
    }

    let previous_seen = data.tag_unigram_count(previous) > 0;
    let current_seen = data.tag_unigram_count(current) > 0;
    if previous_seen && current_seen {
        -18
    } else {
        0
    }
}

fn chunk_min_anchor_len(chunk: &crate::composer::ComposerChunk) -> usize {
    if chunk.kind == crate::composer::ComposerChunkKind::Exact {
        MIN_EXACT_ANCHOR_LEN
    } else {
        MIN_SPAN_LEN
    }
}

fn composer_alignment_delta(composer: &crate::composer::ComposerAnalysis, start: usize, end: usize) -> i32 {
    if composer.chunks.iter().any(|chunk| {
        let len = chunk.end.saturating_sub(chunk.start);
        len < chunk_min_anchor_len(chunk)
    }) {
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

fn exact_anchor_path_bonus(request: &DecodeRequest<'_>) -> i32 {
    let chunks = request.composer.weighted_span_phrase_chunks();
    if chunks.len() < 2
        || !chunks
            .iter()
            .all(|chunk| chunk.kind == crate::composer::ComposerChunkKind::Exact)
    {
        return 0;
    }

    let long_exact = chunks
        .iter()
        .filter(|chunk| chunk.end.saturating_sub(chunk.start) >= 4)
        .count() as i32;
    let short_exact = chunks
        .iter()
        .filter(|chunk| {
            let len = chunk.end.saturating_sub(chunk.start);
            (MIN_EXACT_ANCHOR_LEN..4).contains(&len)
        })
        .count() as i32;
    long_exact * 8_000 + short_exact * 4_000
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
    use std::sync::{Arc, OnceLock};

    use super::{beam_item_to_candidate, context_delta, pos_delta, BeamItem, SpanCandidate, WeightedSpanDecoder};
    use crate::decoder::{DecoderConfig, DecoderMode};
    use crate::roman_lookup::{LegacyData, RankedLexicon, Transliterator};

    fn weighted_span() -> &'static Transliterator {
        static TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
        TRANSLITERATOR.get_or_init(|| {
            let mut config = DecoderConfig::default().with_mode(DecoderMode::Wfst);
            config.wfst_max_latency_ms = u64::MAX;
            Transliterator::from_default_data_with_config(config).unwrap()
        })
    }

    fn test_span(from_model: bool) -> SpanCandidate {
        SpanCandidate {
            start: 0,
            end: 1,
            input: "x".to_owned(),
            output: "ខ".to_owned(),
            recovered_roman: "x".to_owned(),
            first_tag: None,
            last_tag: None,
            score: 0,
            score_bps: 0,
            edit_similarity: 0.0,
            from_model,
            lexicon_verified: true,
        }
    }

    // Provenance: a candidate is flagged `from_model` iff ANY of its spans came from the model
    // provider (so the UI can mark model-assisted suggestions).
    #[test]
    fn candidate_flagged_from_model_when_any_span_is() {
        let with_model = BeamItem {
            spans: vec![test_span(false), test_span(true)],
            ..Default::default()
        };
        assert!(beam_item_to_candidate(with_model, 100, &DecoderConfig::default()).from_model);

        let lexicon_only = BeamItem {
            spans: vec![test_span(false), test_span(false)],
            ..Default::default()
        };
        assert!(!beam_item_to_candidate(lexicon_only, 100, &DecoderConfig::default()).from_model);
    }

    #[test]
    fn model_candidate_absent_from_lexicon_remains_visible_and_unverified() {
        let mut config = DecoderConfig::shadow_interactive()
            .with_mode(DecoderMode::Wfst)
            .with_span_proposal_mode(crate::decoder::SpanProposalMode::StaticTest);
        config.wfst_max_latency_ms = u64::MAX;
        let transliterator = Transliterator::from_default_data_with_config(config).unwrap();

        let candidate = transliterator
            .phrase_candidates("qzx", &HashMap::new())
            .into_iter()
            .find(|candidate| candidate.text == "គហិបតី")
            .expect("the model's out-of-Lexicon suggestion must remain available");

        assert!(candidate.from_model);
        assert!(!candidate.lexicon_verified);
    }

    #[test]
    fn model_proposal_cannot_displace_an_exact_anchored_phrase() {
        let mut config = DecoderConfig::shadow_interactive()
            .with_mode(DecoderMode::Wfst)
            .with_span_proposal_mode(crate::decoder::SpanProposalMode::StaticTest);
        config.wfst_max_latency_ms = u64::MAX;
        let transliterator = Transliterator::from_default_data_with_config(config).unwrap();

        let candidates = transliterator.phrase_candidates("novpeldael", &HashMap::new());

        assert_eq!(
            candidates.first().map(|candidate| candidate.text.as_str()),
            Some("នៅពេលដែល"),
            "three exact Lexicon chunks must remain ahead of an unrelated whole-word model proposal; candidates={candidates:?}"
        );
        assert!(
            candidates
                .iter()
                .find(|candidate| candidate.text == "បច្ចុប្បន្នភាព")
                .is_some_and(|candidate| candidate.from_model),
            "an alternative also proposed by the model must retain ✦ provenance after text deduplication; candidates={candidates:?}"
        );
    }

    #[test]
    fn returns_candidates_for_exact_entries() {
        assert_eq!(
            weighted_span()
                .suggest("jea", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("ជា")
        );
    }

    #[test]
    fn composes_exact_chunk_phrases_without_spaces() {
        assert_eq!(
            weighted_span()
                .suggest("khnhomttov", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("ខ្ញុំទៅ")
        );
    }

    #[test]
    fn recovers_noisy_long_phrase_with_beam_search() {
        assert_eq!(
            weighted_span()
                .suggest("knhhomttovsalarien", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("ខ្ញុំទៅសាលារៀន")
        );
    }

    #[test]
    fn prefers_long_meaningful_span_over_fragmentation() {
        assert_eq!(
            weighted_span()
                .suggest("saensronors", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("សែនស្រណោះ")
        );
    }

    #[test]
    fn prefers_exact_phrase_chunks_over_fuzzy_fragmentation() {
        assert_eq!(
            weighted_span()
                .suggest("sakampheapttenglay", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("សកម្មភាពទាំងឡាយ")
        );
    }

    #[test]
    fn avoids_english_name_fragmentation_inside_khmer_phrase() {
        assert_eq!(
            weighted_span()
                .suggest("meannekbongtte", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("មាននឹកបងទេ")
        );
    }

    #[test]
    fn trusts_common_short_exact_phrase_anchors() {
        assert_eq!(
            weighted_span()
                .suggest("gettengos", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("គេទាំងអស់")
        );
    }

    #[test]
    fn recovers_kit_alias_inside_long_phrase() {
        assert_eq!(
            weighted_span()
                .suggest("khomtaekitmnakaeng", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("ខំតែគិតម្នាក់ឯង")
        );
    }

    #[test]
    fn recovers_invoice_aliases_inside_phrases() {
        for (input, expected) in [
            ("vikkaybat", "វិក្កយបត្រ"),
            ("tvervikaibat", "ធ្វើវិក្កយបត្រ"),
            ("tvervikkaybat", "ធ្វើវិក្កយបត្រ"),
            ("ngaitvervikkaybat", "ថ្ងៃធ្វើវិក្កយបត្រ"),
        ] {
            assert_eq!(
                weighted_span()
                    .suggest(input, &HashMap::new())
                    .first()
                    .map(String::as_str),
                Some(expected),
                "{input}"
            );
        }
    }

    #[test]
    fn single_span_reranker_prefers_sronaoh_variant() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let data = Arc::new(LegacyData::from_entries(transliterator.entries().to_vec()));
        let decoder = WeightedSpanDecoder::new(data, DecoderConfig::default().with_mode(DecoderMode::Wfst));

        let candidates = decoder.rerank_span_candidates(0, 7, "sronors");
        assert_eq!(
            candidates.first().map(|candidate| candidate.output.as_str()),
            Some("ស្រណោះ")
        );
    }

    #[test]
    fn prefers_exact_single_span_match_for_sronos() {
        assert_eq!(
            weighted_span()
                .suggest("sronos", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("ស្រណោះ")
        );
    }

    #[test]
    fn context_delta_prefers_corpus_surface_counts_for_joined_outputs() {
        let mut ranked = RankedLexicon::default();
        ranked.corpus_surface_unigrams.insert(String::from("ខ្ញុំទៅ"), 3);
        ranked.word_unigrams.insert(String::from("ខ្ញុំទៅ"), 1);
        let data = LegacyData::from_ranked_for_tests(ranked);

        assert!(context_delta(&data, None, "ខ្ញុំទៅ") > 0);
    }

    #[test]
    fn pos_delta_prefers_seen_tag_transitions() {
        let mut ranked = RankedLexicon::default();
        ranked.tag_unigrams.insert(String::from("PRO"), 10);
        ranked.tag_unigrams.insert(String::from("VB"), 12);
        ranked.tag_unigrams.insert(String::from("JJ"), 4);
        ranked.tag_bigrams.insert((String::from("PRO"), String::from("VB")), 25);
        let data = LegacyData::from_ranked_for_tests(ranked);

        assert!(pos_delta(&data, Some("PRO"), Some("VB")) > 0);
        assert!(pos_delta(&data, Some("PRO"), Some("JJ")) < 0);
        assert_eq!(pos_delta(&data, None, Some("VB")), 0);
    }
}
