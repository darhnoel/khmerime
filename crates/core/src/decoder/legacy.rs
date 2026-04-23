use std::sync::Arc;

use crate::roman_lookup::LegacyData;

use super::{
    elapsed_decode_us, start_decode_timer, DecodeCandidate, DecodeRequest, DecodeResult, DecodeSegment, Decoder,
};

const PHRASE_CHUNK_LIMIT: usize = 3;
const PHRASE_COMBINATION_LIMIT: usize = 8;

#[derive(Clone)]
pub(crate) struct LegacyDecoder {
    data: Arc<LegacyData>,
}

impl LegacyDecoder {
    pub(crate) fn new(data: Arc<LegacyData>) -> Self {
        Self { data }
    }
}

impl Decoder for LegacyDecoder {
    fn name(&self) -> &'static str {
        "legacy"
    }

    fn decode(&self, request: &DecodeRequest<'_>) -> DecodeResult {
        let started_at = start_decode_timer();
        let mut candidates = Vec::new();
        if request.composer.is_multi_chunk() {
            candidates.extend(self.compose_phrase_candidates(request));
        }
        for candidate in self
            .data
            .suggest(request.input, request.history)
            .into_iter()
            .map(|text| DecodeCandidate {
                text,
                score_bps: None,
                confidence_bps: None,
                segments: Vec::new(),
            })
        {
            if !candidates.iter().any(|current| current.text == candidate.text) {
                candidates.push(candidate);
            }
        }
        DecodeResult::success(self.name(), candidates, elapsed_decode_us(started_at))
    }
}

impl LegacyDecoder {
    fn compose_phrase_candidates(&self, request: &DecodeRequest<'_>) -> Vec<DecodeCandidate> {
        let mut per_chunk = Vec::<Vec<String>>::new();
        for chunk in &request.composer.chunks {
            let mut chunk_candidates = self
                .data
                .exact_targets(&chunk.normalized)
                .map(|targets| targets.iter().take(PHRASE_CHUNK_LIMIT).cloned().collect::<Vec<_>>())
                .unwrap_or_else(|| {
                    self.data
                        .suggest(&chunk.normalized, request.history)
                        .into_iter()
                        .take(PHRASE_CHUNK_LIMIT)
                        .collect::<Vec<_>>()
                });
            chunk_candidates.dedup();
            if chunk_candidates.is_empty() {
                return Vec::new();
            }
            per_chunk.push(chunk_candidates);
        }

        let mut combinations = Vec::new();
        let mut current = Vec::new();
        expand_phrase_combinations(&per_chunk, 0, &mut current, &mut combinations);
        combinations
            .into_iter()
            .take(PHRASE_COMBINATION_LIMIT)
            .map(|parts| {
                let text = parts.concat();
                let segments = request
                    .composer
                    .chunks
                    .iter()
                    .zip(parts.iter())
                    .map(|(chunk, output)| DecodeSegment {
                        input: chunk.normalized.clone(),
                        output: output.clone(),
                        weight_bps: 10_000,
                    })
                    .collect::<Vec<_>>();
                DecodeCandidate {
                    text,
                    score_bps: Some(10_000),
                    confidence_bps: Some(9_200),
                    segments,
                }
            })
            .collect()
    }
}

fn expand_phrase_combinations(
    per_chunk: &[Vec<String>],
    index: usize,
    current: &mut Vec<String>,
    output: &mut Vec<Vec<String>>,
) {
    if output.len() >= PHRASE_COMBINATION_LIMIT {
        return;
    }
    if index == per_chunk.len() {
        output.push(current.clone());
        return;
    }
    for candidate in &per_chunk[index] {
        current.push(candidate.clone());
        expand_phrase_combinations(per_chunk, index + 1, current, output);
        current.pop();
        if output.len() >= PHRASE_COMBINATION_LIMIT {
            return;
        }
    }
}
