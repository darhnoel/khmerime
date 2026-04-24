//! macOS InputMethodKit adapter scaffold.
//!
//! This crate is intentionally a **commented contract skeleton**.
//! It documents how a native InputMethodKit implementation should bridge
//! callback events into shared `khmerime_session` commands.
//!
//! Dioxus is intentionally out-of-scope for this adapter runtime.
//!
//! References:
//! - InputMethodKit overview: <https://developer.apple.com/documentation/inputmethodkit>
//! - IMKInputController: <https://developer.apple.com/documentation/inputmethodkit/imkinputcontroller>

use khmerime_session::{CursorLocation, NativeKeyEvent, SessionCommand, SessionResult, SessionSnapshot};

/// Logical callback surface from `IMKInputController` style lifecycle/events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MacosImkCallback {
    /// Input controller became active for a text client.
    ActivateServer,
    /// Input controller deactivated for client/session.
    DeactivateServer,
    /// Keystroke routed through InputMethodKit event handling.
    HandleEvent(NativeKeyEvent),
    /// Client cursor rectangle changed.
    CursorRectChanged(CursorLocation),
    /// Explicit cancel operation (Esc or controller reset).
    CancelComposition,
}

/// Lifecycle mapping row used by docs/tests to keep callback expectations explicit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallbackMapping {
    pub callback: &'static str,
    pub session_intent: &'static str,
    pub notes: &'static str,
}

const CALLBACK_MAP: &[CallbackMapping] = &[
    CallbackMapping {
        callback: "activateServer:",
        session_intent: "focus_in",
        notes: "activate IME session for focused text client",
    },
    CallbackMapping {
        callback: "deactivateServer:",
        session_intent: "focus_out",
        notes: "close composition and flush temporary state",
    },
    CallbackMapping {
        callback: "handleEvent:",
        session_intent: "process_key_event",
        notes: "main key-processing path",
    },
    CallbackMapping {
        callback: "setMarkedText/selection changes",
        session_intent: "set_cursor_location + snapshot refresh",
        notes: "used for candidate anchor and segment preview alignment",
    },
];

/// Minimal render responsibilities from session snapshot/result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MacosRenderState {
    /// Candidate rows for macOS candidate window.
    pub candidates: Vec<String>,
    /// Marked/preedit text to expose as in-progress composition.
    pub preedit: String,
    /// Optional committed text to insert into client document.
    pub commit_text: Option<String>,
}

/// Returns static callback mapping for docs and contract tests.
pub fn callback_map() -> &'static [CallbackMapping] {
    CALLBACK_MAP
}

/// Converts callback intent to session command.
///
/// Implemented later once Objective-C/Swift InputMethodKit layer is wired.
pub fn map_callback_to_session_command(_callback: &MacosImkCallback) -> Option<SessionCommand> {
    // Implemented later:
    // - Activate/deactivate lifecycle.
    // - Key event conversion.
    // - Cursor updates for candidate positioning.
    None
}

/// Derives adapter-owned render responsibilities from session output.
pub fn derive_render_state(snapshot: &SessionSnapshot, result: &SessionResult) -> MacosRenderState {
    MacosRenderState {
        candidates: snapshot.candidates.clone(),
        preedit: snapshot.preedit.clone(),
        commit_text: result.commit_text.clone(),
    }
}
