//! Future TSF runtime boundary.
//!
//! These modules are placeholders for Windows-only TSF integration. They should
//! remain thin wrappers around TSF callbacks, edit sessions, composition ranges,
//! and candidate display state.

pub mod candidates;
pub mod composition;
pub mod display_attributes;
pub mod edit_session;
pub mod key_event_sink;
pub mod thread_context;
