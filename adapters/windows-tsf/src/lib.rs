//! Windows TSF adapter scaffold.
//!
//! This crate is intentionally a **commented contract skeleton**.
//! It documents how a Windows Text Services Framework (TSF) IME should map
//! native callbacks/sinks to shared `khmerime_session` commands.
//!
//! Dioxus runtime code is intentionally excluded from this adapter.
//! TSF code must use `khmerime_session::ImeSession` as the IME boundary and
//! must not call `khmerime_core` directly.
//!
//! This crate is still a skeleton. It intentionally does not export a COM DLL,
//! register a TSF text service, mutate TSF document ranges, or render candidate
//! UI yet.
//!
//! References:
//! - IME overview: <https://learn.microsoft.com/en-us/windows/apps/develop/input/input-method-editors>
//! - IME requirements: <https://learn.microsoft.com/en-us/windows/apps/develop/input/input-method-editor-requirements>
//! - ITfTextInputProcessor: <https://learn.microsoft.com/en-us/windows/win32/api/msctf/nn-msctf-itftextinputprocessor>

use khmerime_session::{CursorLocation, NativeKeyEvent, SessionCommand, SessionResult, SessionSnapshot};

#[cfg(windows)]
pub mod com;
pub mod history;
pub mod input;
pub mod render;
pub mod session_driver;
#[cfg(windows)]
pub mod tsf;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_map_keeps_tsf_boundaries_visible() {
        let callbacks: Vec<_> = callback_map().iter().map(|mapping| mapping.callback).collect();

        assert!(callbacks.contains(&"ITfTextInputProcessor::Activate"));
        assert!(callbacks.contains(&"ITfTextInputProcessor::Deactivate"));
        assert!(callbacks.contains(&"ITfKeyEventSink::OnKeyDown"));
    }

    #[test]
    fn callback_mapping_is_still_unimplemented_in_skeleton_phase() {
        assert_eq!(map_callback_to_session_command(&WindowsTsfCallback::Activate), None);
    }

    #[test]
    fn render_state_mirrors_session_snapshot_and_result() {
        let snapshot = SessionSnapshot {
            preedit: "ជា".to_owned(),
            candidates: vec!["ជា".to_owned(), "ជ".to_owned()],
            ..SessionSnapshot::default()
        };
        let result = SessionResult {
            commit_text: Some("ជា".to_owned()),
            ..SessionResult::default()
        };

        let render_state = derive_render_state(&snapshot, &result);

        assert_eq!(render_state.preedit, "ជា");
        assert_eq!(render_state.candidates, vec!["ជា", "ជ"]);
        assert_eq!(render_state.commit_text.as_deref(), Some("ជា"));
    }

    #[test]
    fn skeleton_exports_future_module_boundaries() {
        assert!(!input::key_convert::KEY_CONVERSION_IMPLEMENTED);
        assert_eq!(
            session_driver::FIRST_IMPLEMENTATION_MILESTONE,
            "pure Rust Windows session driver around ImeSession"
        );
        assert_eq!(history::PLANNED_HISTORY_PATH, "%APPDATA%\\khmerime\\history.tsv");
    }
}
