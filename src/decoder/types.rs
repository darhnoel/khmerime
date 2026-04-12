use std::collections::HashMap;

use crate::composer::ComposerAnalysis;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecoderMode {
    Legacy,
    Shadow,
    Wfst,
    Hybrid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeFailure {
    EmptyResult,
    Error(String),
    LowConfidence,
    Timeout,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeSegment {
    pub input: String,
    pub output: String,
    pub weight_bps: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecodeCandidate {
    pub text: String,
    pub score_bps: Option<u16>,
    pub confidence_bps: Option<u16>,
    pub segments: Vec<DecodeSegment>,
}

#[derive(Clone, Debug)]
pub struct DecodeRequest<'a> {
    pub input: &'a str,
    pub history: &'a HashMap<String, usize>,
    pub(crate) composer: &'a ComposerAnalysis,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecodeResult {
    pub decoder: &'static str,
    pub candidates: Vec<DecodeCandidate>,
    pub failure: Option<DecodeFailure>,
    pub latency_us: u64,
}

impl DecodeResult {
    pub fn success(decoder: &'static str, candidates: Vec<DecodeCandidate>, latency_us: u64) -> Self {
        let failure = if candidates.is_empty() {
            Some(DecodeFailure::EmptyResult)
        } else {
            None
        };
        Self {
            decoder,
            candidates,
            failure,
            latency_us,
        }
    }

    pub fn failed(decoder: &'static str, failure: DecodeFailure, latency_us: u64) -> Self {
        Self {
            decoder,
            candidates: Vec::new(),
            failure: Some(failure),
            latency_us,
        }
    }

    pub fn visible_strings(&self) -> Vec<String> {
        self.candidates.iter().map(|candidate| candidate.text.clone()).collect()
    }
}
