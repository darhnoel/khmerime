//! Windows TSF adapter scaffold.
//!
//! This crate is intentionally a **commented contract skeleton**.
//! It documents how a Windows Text Services Framework (TSF) IME should map
//! native callbacks/sinks to shared `khmerime_session` commands.
//!
//! Dioxus runtime code is intentionally excluded from this adapter.
//!
//! References:
//! - IME overview: <https://learn.microsoft.com/en-us/windows/apps/develop/input/input-method-editors>
//! - IME requirements: <https://learn.microsoft.com/en-us/windows/apps/develop/input/input-method-editor-requirements>
//! - ITfTextInputProcessor: <https://learn.microsoft.com/en-us/windows/win32/api/msctf/nn-msctf-itftextinputprocessor>

use khmerime_session::{CursorLocation, NativeKeyEvent, SessionCommand, SessionResult, SessionSnapshot};

/// Logical callback surface from TSF COM interfaces and sinks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowsTsfCallback {
    /// `ITfTextInputProcessor::Activate`.
    Activate,
    /// `ITfTextInputProcessor::Deactivate`.
    Deactivate,
    /// Key event sink callback (`OnKeyDown`/`OnTestKeyDown`).
    KeyDown(NativeKeyEvent),
    /// Client cursor rectangle changed.
    CursorRectChanged(CursorLocation),
    /// External reset request from compartment/profile change.
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
        callback: "ITfTextInputProcessor::Activate",
        session_intent: "focus_in + enable",
        notes: "initialize active text-service session",
    },
    CallbackMapping {
        callback: "ITfTextInputProcessor::Deactivate",
        session_intent: "focus_out + disable",
        notes: "tear down session and clear preedit",
    },
    CallbackMapping {
        callback: "ITfKeyEventSink::OnKeyDown",
        session_intent: "process_key_event",
        notes: "main transliteration key path",
    },
    CallbackMapping {
        callback: "ITfContextView::GetTextExt / selection updates",
        session_intent: "set_cursor_location",
        notes: "anchor candidate and segment preview UI",
    },
];

/// Minimal render responsibilities from session snapshot/result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowsRenderState {
    /// Candidate list for TSF candidate UI.
    pub candidates: Vec<String>,
    /// Composition string for marked/preedit text.
    pub preedit: String,
    /// Optional commit text to finalize in host document.
    pub commit_text: Option<String>,
}

/// Returns static callback mapping for docs and contract tests.
pub fn callback_map() -> &'static [CallbackMapping] {
    CALLBACK_MAP
}

/// Converts callback intent to session command.
///
/// Implemented later once COM glue (`msctf`) is added.
pub fn map_callback_to_session_command(_callback: &WindowsTsfCallback) -> Option<SessionCommand> {
    // Implemented later:
    // - wire Activate/Deactivate lifecycle to session focus/enable state.
    // - convert virtual-key events to NativeKeyEvent.
    // - update cursor geometry for candidate anchoring.
    None
}

/// Derives adapter-owned render responsibilities from session output.
pub fn derive_render_state(snapshot: &SessionSnapshot, result: &SessionResult) -> WindowsRenderState {
    WindowsRenderState {
        candidates: snapshot.candidates.clone(),
        preedit: snapshot.preedit.clone(),
        commit_text: result.commit_text.clone(),
    }
}
