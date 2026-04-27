//! Platform-neutral native IME session layer.
//!
//! This crate owns composition state, key semantics, candidate selection,
//! commit/cancel behavior, cursor geometry, and history persistence contracts.
//! Native adapters should translate platform callbacks into `SessionCommand`
//! values and render `SessionSnapshot`/`SessionResult`; they should not duplicate
//! romanization or ranking logic.

mod ime_session;

pub use crate::ime_session::{
    CursorLocation, HistoryStore, ImeSession, ImeSessionSnapshot, ImeSessionUpdate, NativeKeyEvent,
    SegmentPreviewEntry, SessionCommand, SessionResult, SessionSnapshot,
};
