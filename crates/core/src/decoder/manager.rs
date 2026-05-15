use std::collections::HashMap;

use crate::composer::ComposerTable;

use super::{
    build_shadow_observation, DecodeFailure, DecodeRequest, DecodeResult, Decoder, DecoderConfig, DecoderMode,
    LegacyDecoder, ShadowObservation, ShadowReport, WeightedSpanDecoder,
};

pub(crate) struct DecoderManager {
    composer: ComposerTable,
    legacy: LegacyDecoder,
    weighted_span: Option<WeightedSpanDecoder>,
    config: DecoderConfig,
}

impl DecoderManager {
    pub(crate) fn new(
        composer: ComposerTable,
        legacy: LegacyDecoder,
        weighted_span: Option<WeightedSpanDecoder>,
        config: DecoderConfig,
    ) -> Self {
        Self {
            composer,
            legacy,
            weighted_span,
            config,
        }
    }

    pub(crate) fn suggest(&self, input: &str, history: &HashMap<String, usize>) -> Vec<String> {
        let composer = self.composer.analyze(input);
        let request = DecodeRequest {
            input,
            history,
            composer: &composer,
        };
        let legacy = self.legacy.decode(&request);
        let needs_wfst_for_visible = self.weighted_span_needed_for_visible();
        let needs_wfst_for_logging =
            self.config.shadow_log && self.config.mode != DecoderMode::Legacy && self.should_sample_shadow(input);
        let weighted_span = if needs_wfst_for_visible || needs_wfst_for_logging {
            self.decode_weighted_span(&request)
        } else {
            None
        };
        if needs_wfst_for_logging {
            let observation =
                build_shadow_observation(self.config.mode, input, &composer, &legacy, weighted_span.as_ref());
            self.log_shadow_report(&observation);
        }

        let mut visible = self.choose_visible_result(&legacy, weighted_span.as_ref());
        visible.candidates.truncate(self.config.max_candidates);
        let mut suggestions = visible.visible_strings();
        append_raw_query_fallback_tail(&mut suggestions, input, &legacy, self.config.max_candidates);
        suggestions
    }

    fn weighted_span_needed_for_visible(&self) -> bool {
        matches!(self.config.mode, DecoderMode::Wfst | DecoderMode::Hybrid)
    }

    pub(crate) fn shadow_observation(&self, input: &str, history: &HashMap<String, usize>) -> ShadowObservation {
        let composer = self.composer.analyze(input);
        let request = DecodeRequest {
            input,
            history,
            composer: &composer,
        };
        let legacy = self.legacy.decode(&request);
        let weighted_span = self.decode_weighted_span(&request);
        build_shadow_observation(self.config.mode, input, &composer, &legacy, weighted_span.as_ref())
    }

    fn decode_weighted_span(&self, request: &DecodeRequest<'_>) -> Option<DecodeResult> {
        self.weighted_span
            .as_ref()
            .map(|decoder| self.guard_weighted_span_result(decoder.decode(request)))
    }

    fn log_shadow_report(&self, observation: &ShadowObservation) {
        if !self.config.shadow_log {
            return;
        }
        eprintln!("{}", ShadowReport { observation }.format());
    }

    fn should_sample_shadow(&self, input: &str) -> bool {
        if self.config.shadow_sample_bps >= 10_000 {
            return true;
        }
        if self.config.shadow_sample_bps == 0 {
            return false;
        }
        stable_sample_bucket(input) < u64::from(self.config.shadow_sample_bps)
    }

    fn guard_weighted_span_result(&self, mut result: DecodeResult) -> DecodeResult {
        if result.failure.is_some() {
            return result;
        }
        if result.latency_us > self.config.wfst_max_latency_ms.saturating_mul(1_000) {
            result.failure = Some(DecodeFailure::Timeout);
            return result;
        }

        let confidence_ok = result
            .candidates
            .first()
            .and_then(|candidate| candidate.confidence_bps)
            .map(|confidence| confidence >= self.config.wfst_min_confidence_bps)
            .unwrap_or(false);

        if !confidence_ok {
            result.failure = Some(DecodeFailure::LowConfidence);
        }

        result
    }

    fn choose_visible_result(&self, legacy: &DecodeResult, weighted_span: Option<&DecodeResult>) -> DecodeResult {
        match self.config.mode {
            DecoderMode::Legacy | DecoderMode::Shadow => legacy.clone(),
            DecoderMode::Wfst => weighted_span
                .filter(|result| result.failure.is_none() && !result.candidates.is_empty())
                .cloned()
                .unwrap_or_else(|| legacy.clone()),
            DecoderMode::Hybrid => merge_results(legacy, weighted_span, self.config.max_candidates),
        }
    }
}

fn append_raw_query_fallback_tail(
    suggestions: &mut Vec<String>,
    input: &str,
    legacy: &DecodeResult,
    max_candidates: usize,
) {
    if max_candidates == 0 {
        return;
    }

    let query = input.strip_suffix(' ').unwrap_or(input);
    if query.is_empty() || !query.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return;
    }

    let Some(raw_literal) = legacy.candidates.last().map(|candidate| candidate.text.as_str()) else {
        return;
    };
    if raw_literal != query {
        return;
    }

    suggestions.retain(|candidate| candidate != raw_literal);
    if suggestions.len() >= max_candidates {
        suggestions.truncate(max_candidates - 1);
    }
    suggestions.push(raw_literal.to_owned());
}

fn merge_results(legacy: &DecodeResult, weighted_span: Option<&DecodeResult>, limit: usize) -> DecodeResult {
    let mut merged = DecodeResult {
        decoder: "hybrid",
        candidates: Vec::new(),
        failure: None,
        latency_us: legacy.latency_us + weighted_span.map(|result| result.latency_us).unwrap_or(0),
    };

    for candidate in weighted_span
        .filter(|result| result.failure.is_none())
        .into_iter()
        .flat_map(|result| result.candidates.iter())
        .chain(legacy.candidates.iter())
    {
        if !merged.candidates.iter().any(|current| current.text == candidate.text) {
            merged.candidates.push(candidate.clone());
        }
        if merged.candidates.len() >= limit {
            break;
        }
    }

    if merged.candidates.is_empty() {
        merged.failure = Some(
            weighted_span
                .and_then(|result| result.failure.clone())
                .unwrap_or(DecodeFailure::EmptyResult),
        );
    }

    merged
}

fn stable_sample_bucket(input: &str) -> u64 {
    let mut hash = 1_469_598_103_934_665_603u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash % 10_000
}
