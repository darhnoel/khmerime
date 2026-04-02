use super::DecoderMode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecoderConfig {
    pub mode: DecoderMode,
    pub max_candidates: usize,
    pub shadow_log: bool,
    pub shadow_sample_bps: u16,
    pub wfst_min_confidence_bps: u16,
    pub wfst_max_latency_ms: u64,
}

impl DecoderConfig {
    pub fn legacy() -> Self {
        Self::default()
    }

    pub fn with_mode(mut self, mode: DecoderMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_shadow_log(mut self, shadow_log: bool) -> Self {
        self.shadow_log = shadow_log;
        self
    }

    pub fn with_shadow_sample_bps(mut self, shadow_sample_bps: u16) -> Self {
        self.shadow_sample_bps = shadow_sample_bps.min(10_000);
        self
    }
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            mode: DecoderMode::Legacy,
            max_candidates: 10,
            shadow_log: false,
            shadow_sample_bps: 10_000,
            wfst_min_confidence_bps: 6500,
            wfst_max_latency_ms: 10,
        }
    }
}
