mod composer;
mod decoder;
mod roman_lookup;

pub use crate::decoder::{
    suggest_manual_character_candidates, DecodeCandidate, DecodeFailure, DecodeRequest, DecodeResult, DecodeSegment,
    DecoderConfig, DecoderMode, ManualComposeCandidate, ManualComposeKind, ShadowMismatch, ShadowObservation,
    ShadowSummary,
};
pub use crate::roman_lookup::{AppliedSuggestion, Entry, LexiconError, Result, Transliterator};
