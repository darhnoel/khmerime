//! Placeholder for the future `ITfTextInputProcessor` implementation.
//!
//! The text service will own TSF activation/deactivation, thread-manager state,
//! client id, and sink registration. It should delegate key handling to the TSF
//! key-event sink and delegate IME behavior to `session_driver`.

/// Planned lifecycle callbacks for the TSF text service shell.
pub const TEXT_SERVICE_LIFECYCLE: &[&str] = &["Activate", "Deactivate"];
