use std::sync::Arc;

use crate::roman_lookup::LegacyData;

use super::{elapsed_decode_us, start_decode_timer, DecodeCandidate, DecodeRequest, DecodeResult, Decoder};

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
        let candidates = self
            .data
            .suggest(request.input, request.history)
            .into_iter()
            .map(|text| DecodeCandidate {
                text,
                score_bps: None,
                confidence_bps: None,
                segments: Vec::new(),
            })
            .collect();
        DecodeResult::success(self.name(), candidates, elapsed_decode_us(started_at))
    }
}
