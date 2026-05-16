//! Platform-neutral native IME session layer.
//!
//! This crate owns composition state, key semantics, candidate selection,
//! commit/cancel behavior, cursor geometry, and history persistence contracts.
//! Native adapters should translate platform callbacks into `SessionCommand`
//! values and render `SessionSnapshot`/`SessionResult`; they should not duplicate
//! romanization or ranking logic.

mod ime_session;
mod nida_keymap;

pub use crate::ime_session::{
    CandidateDisplayEntry, CursorLocation, HistoryStore, ImeSession, ImeSessionOptions, ImeSessionSnapshot,
    ImeSessionUpdate, InputMode, NativeKeyEvent, SegmentPreviewEntry, SegmentedPreviewMode, SessionCommand,
    SessionResult, SessionSnapshot,
};
