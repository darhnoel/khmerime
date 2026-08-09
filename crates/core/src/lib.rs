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
    candidate_span_ends, register_span_proposal_provider, span_proposal_provider_is_registered,
    suggest_manual_character_candidates, DecodeCandidate, DecodeFailure, DecodeRequest, DecodeResult, DecodeSegment,
    DecoderConfig, DecoderMode, ManualComposeCandidate, ManualComposeKind, ShadowMismatch, ShadowObservation,
    ShadowSummary, SpanProposal, SpanProposalMode, SpanProposalProvider, SpanProposalRequest,
};
// Test-only instrumentation: a counter of how many times the Weighted Span decoder ran. Exported so
// tests in dependent crates (e.g. `khmerime_session`) can assert the per-keystroke decode budget —
// one recompute must not decode twice. Not part of the engine's real API; do not call from adapters.
pub use crate::decoder::{reset_weighted_span_decode_calls, weighted_span_decode_calls};
pub use crate::roman_lookup::{
    AppliedSuggestion, Entry, LexiconError, Result, SharedTransliteratorData, Transliterator,
};
pub use crate::segment_refine::{
    build_segmented_session, connect_khmer_display, move_session_focus, normalize_visible_suggestions,
    normalized_suggestion_key, reflow_segmented_session_from_selection, SegmentedChoice, SegmentedSession,
};
pub use crate::utils::khnormal;
pub use khmerime_config::{normalize_pack_key, LexiconPack};
