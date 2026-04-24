//! Android IME adapter scaffold.
//!
//! This crate is intentionally a **commented contract skeleton**.
//! It documents how Android `InputMethodService` callbacks should map into
//! shared `khmerime_session` commands.
//!
//! Dioxus runtime code is intentionally excluded from this adapter.
//!
//! References:
//! - Create an input method:
//!   <https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method>
//! - InputMethodService API:
//!   <https://developer.android.com/reference/android/inputmethodservice/InputMethodService>

use khmerime_session::{CursorLocation, NativeKeyEvent, SessionCommand, SessionResult, SessionSnapshot};

/// Logical callback surface from Android `InputMethodService` lifecycle/events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidImeCallback {
    /// `onStartInput(EditorInfo, restarting)`.
    StartInput,
    /// `onFinishInput()`.
    FinishInput,
    /// Soft/hardware key event forwarded from the Android service.
    KeyInput(NativeKeyEvent),
    /// Cursor/selection update from `onUpdateSelection(...)`.
    CursorLocationChanged(CursorLocation),
    /// Reset/cancel requested by framework or keyboard action.
    ResetRequested,
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
        callback: "onStartInput",
        session_intent: "focus_in + enable",
        notes: "start composition session for focused editor",
    },
    CallbackMapping {
        callback: "onFinishInput",
        session_intent: "focus_out",
        notes: "end composition session and clear preedit",
    },
    CallbackMapping {
        callback: "onKeyDown/onKeyUp or key press handler",
        session_intent: "process_key_event",
        notes: "main transliteration key path",
    },
    CallbackMapping {
        callback: "onUpdateSelection",
        session_intent: "set_cursor_location",
        notes: "update candidate anchor and segment preview placement",
    },
];

/// Minimal render responsibilities from session snapshot/result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AndroidRenderState {
    /// Candidate rows shown by IME candidate strip/panel.
    pub candidates: Vec<String>,
    /// In-progress composition text.
    pub preedit: String,
    /// Optional commit text for `InputConnection.commitText`.
    pub commit_text: Option<String>,
}

/// Returns static callback mapping for docs and contract tests.
pub fn callback_map() -> &'static [CallbackMapping] {
    CALLBACK_MAP
}

/// Converts callback intent to session command.
///
/// Implemented later once Kotlin/Java service boundary is introduced.
pub fn map_callback_to_session_command(_callback: &AndroidImeCallback) -> Option<SessionCommand> {
    // Implemented later:
    // - map lifecycle callbacks to FocusIn/FocusOut/Enable.
    // - convert Android key events to NativeKeyEvent.
    // - propagate cursor updates for candidate anchoring.
    None
}

/// Derives adapter-owned render responsibilities from session output.
pub fn derive_render_state(snapshot: &SessionSnapshot, result: &SessionResult) -> AndroidRenderState {
    AndroidRenderState {
        candidates: snapshot.candidates.clone(),
        preedit: snapshot.preedit.clone(),
        commit_text: result.commit_text.clone(),
    }
}
