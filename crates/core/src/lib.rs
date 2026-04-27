//! Shared KhmerIME transliteration engine.
//!
//! This crate owns roman normalization, lexicon lookup, decoder orchestration,
//! phrase segmentation, Khmer normalization, and compiled data loading. Platform
//! adapters and UI crates should call this through `khmerime_session` unless
//! they are inspection tools such as the CLI.

mod composer;
mod decoder;
mod roman_lookup;
mod segment_refine;
mod utils;

pub use crate::decoder::{
    suggest_manual_character_candidates, DecodeCandidate, DecodeFailure, DecodeRequest, DecodeResult, DecodeSegment,
    DecoderConfig, DecoderMode, ManualComposeCandidate, ManualComposeKind, ShadowMismatch, ShadowObservation,
    ShadowSummary,
};
pub use crate::roman_lookup::{AppliedSuggestion, Entry, LexiconError, Result, Transliterator};
pub use crate::segment_refine::{
    build_segmented_session, connect_khmer_display, move_session_focus, normalize_visible_suggestions,
    normalized_suggestion_key, reflow_segmented_session_from_selection, SegmentedChoice, SegmentedSession,
};
pub use crate::utils::khnormal;
