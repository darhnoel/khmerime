use std::collections::HashMap;

use crate::composer::ComposerTable;

use super::{
    build_shadow_observation, DecodeFailure, DecodeRequest, DecodeResult, Decoder, DecoderConfig, DecoderMode,
    LegacyDecoder, ShadowObservation, ShadowReport, WfstDecoder,
};

pub(crate) struct DecoderManager {
    composer: ComposerTable,
    legacy: LegacyDecoder,
    wfst: Option<WfstDecoder>,
    config: DecoderConfig,
}

impl DecoderManager {
    pub(crate) fn new(
        composer: ComposerTable,
        legacy: LegacyDecoder,
        wfst: Option<WfstDecoder>,
        config: DecoderConfig,
    ) -> Self {
        Self {
            composer,
            legacy,
            wfst,
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
        let wfst = self.decode_wfst(&request);
        let observation = build_shadow_observation(self.config.mode, input, &composer, &legacy, wfst.as_ref());
        if self.config.mode != DecoderMode::Legacy && self.should_sample_shadow(input) {
            self.log_shadow_report(&observation);
        }

        let mut visible = self.choose_visible_result(&legacy, wfst.as_ref());
        visible.candidates.truncate(self.config.max_candidates);
        visible.visible_strings()
    }

    pub(crate) fn shadow_observation(&self, input: &str, history: &HashMap<String, usize>) -> ShadowObservation {
        let composer = self.composer.analyze(input);
        let request = DecodeRequest {
            input,
            history,
            composer: &composer,
        };
        let legacy = self.legacy.decode(&request);
        let wfst = self.decode_wfst(&request);
        build_shadow_observation(self.config.mode, input, &composer, &legacy, wfst.as_ref())
    }

    fn decode_wfst(&self, request: &DecodeRequest<'_>) -> Option<DecodeResult> {
        self.wfst
            .as_ref()
            .map(|decoder| self.guard_wfst_result(decoder.decode(request)))
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

    fn guard_wfst_result(&self, mut result: DecodeResult) -> DecodeResult {
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

    fn choose_visible_result(&self, legacy: &DecodeResult, wfst: Option<&DecodeResult>) -> DecodeResult {
        match self.config.mode {
            DecoderMode::Legacy | DecoderMode::Shadow => legacy.clone(),
            DecoderMode::Wfst => wfst
                .filter(|result| result.failure.is_none() && !result.candidates.is_empty())
                .cloned()
                .unwrap_or_else(|| legacy.clone()),
            DecoderMode::Hybrid => merge_results(legacy, wfst, self.config.max_candidates),
        }
    }
}

fn merge_results(legacy: &DecodeResult, wfst: Option<&DecodeResult>, limit: usize) -> DecodeResult {
    let mut merged = DecodeResult {
        decoder: "hybrid",
        candidates: Vec::new(),
        failure: None,
        latency_us: legacy.latency_us + wfst.map(|result| result.latency_us).unwrap_or(0),
    };

    for candidate in wfst
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
            wfst.and_then(|result| result.failure.clone())
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
