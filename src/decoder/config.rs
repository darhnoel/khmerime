use super::DecoderMode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecoderConfig {
    pub mode: DecoderMode,
    pub max_candidates: usize,
    pub shadow_log: bool,
    pub shadow_sample_bps: u16,
    pub wfst_min_confidence_bps: u16,
    pub wfst_max_latency_ms: u64,
    pub beam_max_span_len: usize,
    pub beam_lengths_per_start: usize,
    pub beam_retrieval_shortlist: usize,
    pub beam_candidates_per_span: usize,
    pub beam_width: usize,
    pub chunk_score_weight: i32,
    pub segmentation_score_weight: i32,
    pub lm_score_weight: i32,
    pub pos_score_weight: i32,
    pub history_score_weight: i32,
    pub interactive_mode: bool,
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

    pub fn shadow_interactive() -> Self {
        Self {
            mode: DecoderMode::Shadow,
            max_candidates: 10,
            shadow_log: false,
            shadow_sample_bps: 10_000,
            wfst_min_confidence_bps: 6500,
            wfst_max_latency_ms: 250,
            beam_max_span_len: 6,
            beam_lengths_per_start: 4,
            beam_retrieval_shortlist: 8,
            beam_candidates_per_span: 2,
            beam_width: 2,
            chunk_score_weight: 1,
            segmentation_score_weight: 1,
            lm_score_weight: 0,
            pos_score_weight: 0,
            history_score_weight: 1,
            interactive_mode: true,
        }
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
            wfst_max_latency_ms: 60_000,
            beam_max_span_len: 10,
            beam_lengths_per_start: 10,
            beam_retrieval_shortlist: 24,
            beam_candidates_per_span: 4,
            beam_width: 6,
            chunk_score_weight: 1,
            segmentation_score_weight: 1,
            lm_score_weight: 1,
            pos_score_weight: 0,
            history_score_weight: 1,
            interactive_mode: false,
        }
    }
}
