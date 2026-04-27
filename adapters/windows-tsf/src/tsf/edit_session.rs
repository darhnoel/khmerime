//! Placeholder for TSF edit-session wrappers.
//!
//! Real text mutation must happen through TSF edit sessions. Future composition
//! updates, commits, and range replacement should be routed through this module
//! instead of being scattered across callback implementations.

/// Planned mutation responsibilities for edit sessions.
pub const EDIT_SESSION_RESPONSIBILITIES: &[&str] = &[
    "update composition text",
    "commit selected text once",
    "end composition after commit or reset",
];
