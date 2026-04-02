use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use crate::roman_lookup::{normalize, similarity, Entry};

use super::{
    elapsed_decode_us, start_decode_timer, DecodeCandidate, DecodeFailure, DecodeRequest, DecodeResult,
    DecodeSegment, Decoder,
};

const PREFIX_LIMIT: usize = 64;
const APPROX_RECORD_LIMIT: usize = 48;
const SUBSTRING_MIN_LEN: usize = 4;
const SUBSTRING_MAX_LEN: usize = 12;
const SUBSTRING_WINDOW_LIMIT: usize = 16;
const SUBSTRING_APPROX_WINDOW_LIMIT: usize = 4;
const SEGMENT_LIMIT: usize = 6;
const EARLY_EXIT_SCORE_BPS: u16 = 7_600;

#[derive(Clone)]
struct WfstRecord {
    roman: String,
    target: String,
    normalized: String,
}

#[derive(Default)]
struct TrieNode {
    children: HashMap<char, usize>,
    terminal_records: Vec<usize>,
    prefix_records: Vec<usize>,
}

pub(crate) struct WfstDecoder {
    nodes: Vec<TrieNode>,
    records: Vec<WfstRecord>,
    grams: HashMap<String, Vec<usize>>,
}

impl WfstDecoder {
    pub(crate) fn from_entries(entries: &[Entry]) -> Self {
        let mut decoder = Self {
            nodes: vec![TrieNode::default()],
            records: Vec::with_capacity(entries.len()),
            grams: HashMap::new(),
        };

        for entry in entries {
            let normalized = normalize(&entry.roman);
            if normalized.is_empty() {
                continue;
            }

            let record_index = decoder.records.len();
            decoder.records.push(WfstRecord {
                roman: entry.roman.clone(),
                target: entry.target.clone(),
                normalized: normalized.clone(),
            });
            decoder.add_grams(&normalized, record_index);

            let mut node_index = 0usize;
            decoder.push_prefix_record(node_index, record_index);
            for ch in normalized.chars() {
                let next = if let Some(&child) = decoder.nodes[node_index].children.get(&ch) {
                    child
                } else {
                    let child = decoder.nodes.len();
                    decoder.nodes.push(TrieNode::default());
                    decoder.nodes[node_index].children.insert(ch, child);
                    child
                };
                node_index = next;
                decoder.push_prefix_record(node_index, record_index);
            }
            decoder.nodes[node_index].terminal_records.push(record_index);
        }

        decoder
    }

    fn add_grams(&mut self, normalized: &str, record_index: usize) {
        let grams = char_ngrams(normalized, 2);
        let mut seen = HashSet::new();
        for gram in grams {
            if seen.insert(gram.clone()) {
                self.grams.entry(gram).or_default().push(record_index);
            }
        }
    }

    fn push_prefix_record(&mut self, node_index: usize, record_index: usize) {
        let prefix_records = &mut self.nodes[node_index].prefix_records;
        if prefix_records.len() < PREFIX_LIMIT {
            prefix_records.push(record_index);
        }
    }

    fn exact_or_prefix_candidates(&self, normalized: &str) -> Vec<DecodeCandidate> {
        let mut node_index = 0usize;
        for ch in normalized.chars() {
            let Some(&child) = self.nodes[node_index].children.get(&ch) else {
                return Vec::new();
            };
            node_index = child;
        }

        let mut candidates = Vec::new();
        let mut seen = HashSet::new();

        for &record_index in &self.nodes[node_index].terminal_records {
            let record = &self.records[record_index];
            if seen.insert(record.target.clone()) {
                candidates.push(candidate_from_record(record, 10_000, 9_500));
            }
        }

        for &record_index in &self.nodes[node_index].prefix_records {
            let record = &self.records[record_index];
            if record.normalized == normalized {
                continue;
            }
            let extra = record
                .normalized
                .chars()
                .count()
                .saturating_sub(normalized.chars().count());
            let score_bps = 9_000u16.saturating_sub((extra as u16).saturating_mul(175));
            let confidence_bps = 8_000u16.saturating_sub((extra as u16).saturating_mul(125));
            if score_bps < 6_000 {
                continue;
            }
            if seen.insert(record.target.clone()) {
                candidates.push(candidate_from_record(record, score_bps, confidence_bps));
            }
        }

        candidates.sort_by_key(|candidate| {
            Reverse((
                candidate.score_bps.unwrap_or_default(),
                candidate.confidence_bps.unwrap_or_default(),
            ))
        });
        candidates
    }

    fn approximate_candidates(&self, normalized: &str) -> Vec<DecodeCandidate> {
        let grams = char_ngrams(normalized, 2);
        if grams.is_empty() {
            return Vec::new();
        }

        let mut overlap_counts = HashMap::<usize, usize>::new();
        for gram in grams {
            if let Some(records) = self.grams.get(&gram) {
                for &record_index in records {
                    *overlap_counts.entry(record_index).or_default() += 1;
                }
            }
        }

        if overlap_counts.is_empty() {
            return Vec::new();
        }

        let mut candidate_indices = overlap_counts.into_iter().collect::<Vec<_>>();
        candidate_indices
            .sort_by_key(|(record_index, overlap)| Reverse((*overlap, self.records[*record_index].normalized.len())));

        let mut scored = candidate_indices
            .into_iter()
            .take(APPROX_RECORD_LIMIT)
            .filter_map(|(record_index, _)| {
                let record = &self.records[record_index];
                let similarity = similarity(&record.normalized, normalized);
                if similarity < 0.45 {
                    return None;
                }
                let score_bps = (similarity * 10_000.0).round() as u16;
                let confidence_bps = score_bps.saturating_sub(400);
                Some((score_bps, confidence_bps, record))
            })
            .collect::<Vec<_>>();

        scored.sort_by_key(|(score_bps, _, record)| Reverse((*score_bps, Reverse(record.normalized.len()))));

        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        for (score_bps, confidence_bps, record) in scored {
            if seen.insert(record.target.clone()) {
                candidates.push(candidate_from_record(record, score_bps, confidence_bps));
            }
        }

        candidates
    }

    fn substring_candidates(&self, normalized: &str) -> Vec<DecodeCandidate> {
        let chars = normalized.chars().collect::<Vec<_>>();
        if chars.len() < SUBSTRING_MIN_LEN {
            return Vec::new();
        }

        let max_len = chars.len().min(SUBSTRING_MAX_LEN);
        let mut windows = Vec::<(usize, usize, usize)>::new();
        for len in (SUBSTRING_MIN_LEN..=max_len).rev() {
            let mut starts = (0..=chars.len().saturating_sub(len)).collect::<Vec<_>>();
            starts.sort_by_key(|start| {
                let end = *start + len;
                let edge_distance = (*start).min(chars.len().saturating_sub(end));
                (edge_distance, Reverse(len), *start)
            });
            for start in starts {
                let end = start + len;
                let edge_distance = start.min(chars.len().saturating_sub(end));
                windows.push((start, end, edge_distance));
                if windows.len() >= SUBSTRING_WINDOW_LIMIT {
                    break;
                }
            }
            if windows.len() >= SUBSTRING_WINDOW_LIMIT {
                break;
            }
        }

        let mut best = HashMap::<String, DecodeCandidate>::new();
        let mut approximate_windows_used = 0usize;
        'windows: for (start, end, edge_distance) in windows {
            let window = chars[start..end].iter().collect::<String>();
            let mut candidates = self.exact_or_prefix_candidates(&window);
            let allow_approximate =
                approximate_windows_used < SUBSTRING_APPROX_WINDOW_LIMIT && (edge_distance == 0 || end - start >= 6);
            if candidates.is_empty() && allow_approximate {
                candidates = self.approximate_candidates(&window);
                if !candidates.is_empty() {
                    approximate_windows_used += 1;
                }
            }

            for candidate in candidates.into_iter().take(SEGMENT_LIMIT) {
                let adjusted = adjust_candidate_for_window(&candidate, &window, chars.len(), end - start);
                upsert_candidate(&mut best, adjusted);
            }

            if edge_distance == 0
                && best
                    .values()
                    .any(|candidate| candidate.score_bps.unwrap_or_default() >= EARLY_EXIT_SCORE_BPS)
            {
                break 'windows;
            }
        }

        let mut candidates = best.into_values().collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| {
            Reverse((
                candidate.score_bps.unwrap_or_default(),
                candidate.confidence_bps.unwrap_or_default(),
                candidate.text.chars().count(),
            ))
        });
        candidates
    }
}

impl Decoder for WfstDecoder {
    fn name(&self) -> &'static str {
        "wfst"
    }

    fn decode(&self, request: &DecodeRequest<'_>) -> DecodeResult {
        let started_at = start_decode_timer();
        let normalized = normalize(request.input);
        if normalized.is_empty() {
            return DecodeResult::failed(
                self.name(),
                DecodeFailure::EmptyResult,
                elapsed_decode_us(started_at),
            );
        }

        let mut candidates = self.exact_or_prefix_candidates(&normalized);
        if candidates.is_empty() {
            candidates = self.approximate_candidates(&normalized);
        }
        if candidates.is_empty() || normalized.chars().count() > SUBSTRING_MAX_LEN {
            let substring_candidates = self.substring_candidates(&normalized);
            for candidate in substring_candidates {
                upsert_candidate_list(&mut candidates, candidate);
            }
            candidates.sort_by_key(|candidate| {
                Reverse((
                    candidate.score_bps.unwrap_or_default(),
                    candidate.confidence_bps.unwrap_or_default(),
                    candidate.text.chars().count(),
                ))
            });
        }

        DecodeResult::success(self.name(), candidates, elapsed_decode_us(started_at))
    }
}

fn candidate_from_record(record: &WfstRecord, score_bps: u16, confidence_bps: u16) -> DecodeCandidate {
    DecodeCandidate {
        text: record.target.clone(),
        score_bps: Some(score_bps),
        confidence_bps: Some(confidence_bps),
        segments: vec![DecodeSegment {
            input: record.roman.clone(),
            output: record.target.clone(),
            weight_bps: score_bps,
        }],
    }
}

fn adjust_candidate_for_window(
    candidate: &DecodeCandidate,
    matched_input: &str,
    full_len: usize,
    matched_len: usize,
) -> DecodeCandidate {
    let base_score = candidate.score_bps.unwrap_or(0);
    let base_confidence = candidate.confidence_bps.unwrap_or(base_score);
    let uncovered = full_len.saturating_sub(matched_len) as u16;
    let window_bonus = ((matched_len as u32 * 2_000) / full_len.max(1) as u32) as u16;
    let penalty = uncovered.saturating_mul(120);
    let score_bps = base_score.saturating_sub(penalty).saturating_add(window_bonus);
    let confidence_bps = base_confidence
        .saturating_sub(penalty / 2)
        .saturating_add(window_bonus / 2);

    DecodeCandidate {
        text: candidate.text.clone(),
        score_bps: Some(score_bps),
        confidence_bps: Some(confidence_bps),
        segments: vec![DecodeSegment {
            input: matched_input.to_owned(),
            output: candidate.text.clone(),
            weight_bps: score_bps,
        }],
    }
}

fn upsert_candidate(best: &mut HashMap<String, DecodeCandidate>, candidate: DecodeCandidate) {
    match best.get(candidate.text.as_str()) {
        Some(current) => {
            let current_score = current.score_bps.unwrap_or_default();
            let next_score = candidate.score_bps.unwrap_or_default();
            let current_confidence = current.confidence_bps.unwrap_or_default();
            let next_confidence = candidate.confidence_bps.unwrap_or_default();
            if (next_score, next_confidence) > (current_score, current_confidence) {
                best.insert(candidate.text.clone(), candidate);
            }
        }
        None => {
            best.insert(candidate.text.clone(), candidate);
        }
    }
}

fn upsert_candidate_list(candidates: &mut Vec<DecodeCandidate>, candidate: DecodeCandidate) {
    if let Some(position) = candidates.iter().position(|current| current.text == candidate.text) {
        let current_score = candidates[position].score_bps.unwrap_or_default();
        let next_score = candidate.score_bps.unwrap_or_default();
        let current_confidence = candidates[position].confidence_bps.unwrap_or_default();
        let next_confidence = candidate.confidence_bps.unwrap_or_default();
        if (next_score, next_confidence) > (current_score, current_confidence) {
            candidates[position] = candidate;
        }
    } else {
        candidates.push(candidate);
    }
}

fn char_ngrams(input: &str, size: usize) -> Vec<String> {
    let chars = input.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Vec::new();
    }
    if chars.len() <= size {
        return vec![chars.iter().collect()];
    }

    let mut grams = Vec::with_capacity(chars.len().saturating_sub(size) + 1);
    for start in 0..=chars.len().saturating_sub(size) {
        grams.push(chars[start..start + size].iter().collect());
    }
    grams
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::roman_lookup::Transliterator;

    use super::*;

    #[test]
    fn returns_candidates_for_exact_entries() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let decoder = WfstDecoder::from_entries(transliterator.entries());
        let history = HashMap::new();
        let request = DecodeRequest {
            input: "jea",
            history: &history,
        };

        let result = decoder.decode(&request);
        assert_eq!(
            result.candidates.first().map(|candidate| candidate.text.as_str()),
            Some("ជា")
        );
    }

    #[test]
    fn recovers_embedded_word_from_noisy_long_token() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let decoder = WfstDecoder::from_entries(transliterator.entries());
        let history = HashMap::new();
        let request = DecodeRequest {
            input: "knhoddmtofvvsffalarien",
            history: &history,
        };

        let result = decoder.decode(&request);
        assert!(result.candidates.iter().any(|candidate| candidate.text == "សាលារៀន"));
    }
}
