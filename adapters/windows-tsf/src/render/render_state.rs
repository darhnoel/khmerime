//! Placeholder for future Windows render actions.
//!
//! The current public `derive_render_state` helper in `lib.rs` is the contract
//! skeleton. Future TSF code can replace or extend it with explicit render
//! actions once edit-session mutation is implemented.

/// Render actions expected when the TSF adapter becomes runnable.
pub const PLANNED_RENDER_ACTIONS: &[&str] = &[
    "update preedit composition",
    "refresh candidate list",
    "commit text exactly once",
    "clear composition",
];
