//! Placeholder for per-thread TSF state.
//!
//! Future state belongs here: thread manager, document manager, active context,
//! sink cookies, client id, and focus state. Keep this separate from session
//! behavior so COM lifetime bugs are isolated from KhmerIME logic.

/// Documents the planned thread-context owner.
pub const THREAD_CONTEXT_BOUNDARY: &str = "TSF per-thread context and sink ownership";
