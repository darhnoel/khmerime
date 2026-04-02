use std::collections::HashMap;

use crate::composer::ComposerTable;

#[cfg(feature = "wfst-decoder")]
use super::DecodeFailure;
#[cfg(feature = "wfst-decoder")]
use super::WfstDecoder;
use super::{
    build_shadow_observation, DecodeRequest, DecodeResult, Decoder, DecoderConfig, DecoderMode, LegacyDecoder,
    ShadowObservation, ShadowReport,
};

pub(crate) struct DecoderManager {
    composer: ComposerTable,
    legacy: LegacyDecoder,
    #[cfg(feature = "wfst-decoder")]
    wfst: Option<WfstDecoder>,
    config: DecoderConfig,
}

impl DecoderManager {
    #[cfg(feature = "wfst-decoder")]
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

    #[cfg(not(feature = "wfst-decoder"))]
    pub(crate) fn new(composer: ComposerTable, legacy: LegacyDecoder, config: DecoderConfig) -> Self {
        Self {
            composer,
            legacy,
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

        let mut visible = legacy;
        visible.candidates.truncate(self.config.max_candidates);
        visible.visible_strings()
    }

    pub(crate) fn active_decoder_name(&self) -> &'static str {
        self.legacy.name()
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

    #[cfg(feature = "wfst-decoder")]
    fn decode_wfst(&self, request: &DecodeRequest<'_>) -> Option<DecodeResult> {
        self.wfst
            .as_ref()
            .map(|decoder| self.guard_wfst_result(decoder.decode(request)))
    }

    #[cfg(not(feature = "wfst-decoder"))]
    fn decode_wfst(&self, _: &DecodeRequest<'_>) -> Option<DecodeResult> {
        None
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

    #[cfg(feature = "wfst-decoder")]
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
}

fn stable_sample_bucket(input: &str) -> u64 {
    let mut hash = 1_469_598_103_934_665_603u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash % 10_000
}
