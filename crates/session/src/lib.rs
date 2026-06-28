//! Platform-neutral native IME session layer.
//!
//! This crate owns composition state, key semantics, candidate selection,
//! commit/cancel behavior, cursor geometry, and history persistence contracts.
//! Native adapters should translate platform callbacks into `SessionCommand`
//! values and render `SessionSnapshot`/`SessionResult`; they should not duplicate
//! romanization or ranking logic.

// Adapter-facing boundary types adapters build commands from and render
// snapshots into (no engine internals, no platform handles).
mod adapter_contract;
// The ordered commit-time precedence: Visible Segmented Commit → Visible
// Candidate Commit → Hidden Commit Fallback → raw roman floor.
mod commit_rules;
// `ImeSession` itself: state, constructors, lifecycle, and key-event dispatch
// that routes into the other modules.
mod char_pick;
mod ime_session;
// NIDA input mode: the `Nida` key handler plus its direct-Khmer keymap table.
mod nida;
// Segment Edit Mode: rewriting one focused segment in isolation (Tab/Backspace
// semantics, sibling pinning).
mod segment_edit_mode;
// Session-owned segmented composition model: focus state, per-segment candidate
// selection, visible-suggestion normalization, and reflow.
mod segment_model;
// Segmented Session navigation: focus movement, per-segment candidate cycling,
// and reflow/rebuild from decoder observations.
mod segmented_session;
// Read-only projection of live state into the render-facing `SessionSnapshot`.
mod session_snapshot;
// Shared `#[cfg(test)]` session builders used by every module's test block.
#[cfg(test)]
mod test_support;

pub use crate::adapter_contract::{
    should_persist_history_word, CandidateDisplayEntry, CursorLocation, HistoryStore, ImeSessionOptions,
    ImeSessionSnapshot, ImeSessionUpdate, InputMode, NativeKeyEvent, SegmentPreviewEntry, SegmentedPreviewMode,
    SessionCommand, SessionResult, SessionSnapshot, MAX_PERSISTED_HISTORY_WORD_CHARS,
};
pub use crate::ime_session::{ImeSession, ImeSessionBuilder};
pub use crate::segmented_session::{compute_segmented_refinement, SegmentedRefinement};
