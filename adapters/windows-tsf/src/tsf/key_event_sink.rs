//! Placeholder for the future `ITfKeyEventSink` implementation.
//!
//! `OnTestKeyDown` and `OnKeyDown` must use shared conversion/prediction logic
//! so the host application never receives a key that KhmerIME already consumed.

/// Key sink callbacks expected in the first runnable TSF spike.
pub const KEY_EVENT_SINK_CALLBACKS: &[&str] = &[
    "OnSetFocus",
    "OnTestKeyDown",
    "OnKeyDown",
    "OnTestKeyUp",
    "OnKeyUp",
    "OnPreservedKey",
];
