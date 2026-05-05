//! Windows TSF adapter.
//!
//! The shared IME behavior lives in `khmerime_session`. This crate owns the
//! Windows Text Services Framework boundary: COM registration/lifecycle, key
//! conversion, render-state derivation, and TSF document mutation.

use khmerime_session::{CursorLocation, NativeKeyEvent, SessionCommand, SessionResult, SessionSnapshot};

#[cfg(windows)]
pub mod com;
pub mod diagnostics;
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

/// Render responsibilities from session snapshot/result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowsRenderState {
    /// Whether TSF should eat the original key.
    pub consumed: bool,
    /// Candidate list for TSF candidate UI.
    pub candidates: Vec<String>,
    /// Active candidate index for TSF candidate UI highlighting.
    pub selected_index: Option<usize>,
    /// Composition string for marked/preedit text.
    pub preedit: String,
    /// Optional commit text to finalize in host document.
    pub commit_text: Option<String>,
    /// Current caret/composition rectangle for candidate anchoring.
    pub cursor_location: CursorLocation,
    /// Coarse-grained render actions for the native TSF layer.
    pub actions: Vec<render::RenderAction>,
}

/// Returns static callback mapping for docs and contract tests.
pub fn callback_map() -> &'static [CallbackMapping] {
    CALLBACK_MAP
}

/// Converts callback intent to session command(s).
pub fn map_callback_to_session_commands(callback: &WindowsTsfCallback) -> Vec<SessionCommand> {
    match callback {
        WindowsTsfCallback::Activate => vec![SessionCommand::Enable, SessionCommand::FocusIn],
        WindowsTsfCallback::Deactivate => vec![SessionCommand::FocusOut, SessionCommand::Disable],
        WindowsTsfCallback::KeyDown(event) => vec![SessionCommand::ProcessKeyEvent(*event)],
        WindowsTsfCallback::CursorRectChanged(location) => vec![SessionCommand::SetCursorLocation(*location)],
        WindowsTsfCallback::ResetRequested => vec![SessionCommand::Reset],
    }
}

/// Converts callback intent to the first session command.
///
/// Lifecycle callbacks such as activation and deactivation expand to multiple
/// commands through [`map_callback_to_session_commands`]. This helper preserves
/// the original single-command API for simple callback paths.
pub fn map_callback_to_session_command(callback: &WindowsTsfCallback) -> Option<SessionCommand> {
    map_callback_to_session_commands(callback).into_iter().next()
}

/// Derives adapter-owned render responsibilities from session output.
pub fn derive_render_state(snapshot: &SessionSnapshot, result: &SessionResult) -> WindowsRenderState {
    let mut actions = Vec::new();
    if result.commit_text.is_some() {
        actions.push(render::RenderAction::CommitText);
    }
    if snapshot.preedit.is_empty() {
        actions.push(render::RenderAction::ClearComposition);
    } else {
        actions.push(render::RenderAction::UpdateComposition);
    }
    if snapshot.candidates.is_empty() {
        actions.push(render::RenderAction::ClearCandidates);
    } else {
        actions.push(render::RenderAction::UpdateCandidates);
    }

    WindowsRenderState {
        consumed: result.consumed,
        candidates: snapshot.candidates.clone(),
        selected_index: snapshot.selected_index,
        preedit: snapshot.preedit.clone(),
        commit_text: result.commit_text.clone(),
        cursor_location: snapshot.cursor_location,
        actions,
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
    fn callback_mapping_expands_lifecycle_events() {
        assert_eq!(
            map_callback_to_session_commands(&WindowsTsfCallback::Activate),
            vec![SessionCommand::Enable, SessionCommand::FocusIn]
        );
        assert_eq!(
            map_callback_to_session_commands(&WindowsTsfCallback::Deactivate),
            vec![SessionCommand::FocusOut, SessionCommand::Disable]
        );
        assert_eq!(
            map_callback_to_session_command(&WindowsTsfCallback::ResetRequested),
            Some(SessionCommand::Reset)
        );
    }

    #[test]
    fn render_state_mirrors_session_snapshot_and_result() {
        let snapshot = SessionSnapshot {
            preedit: "chea".to_owned(),
            candidates: vec!["candidate".to_owned(), "chea".to_owned()],
            selected_index: Some(0),
            cursor_location: CursorLocation {
                x: 10,
                y: 20,
                width: 2,
                height: 16,
            },
            ..SessionSnapshot::default()
        };
        let result = SessionResult {
            consumed: true,
            commit_text: Some("candidate".to_owned()),
            ..SessionResult::default()
        };

        let render_state = derive_render_state(&snapshot, &result);

        assert!(render_state.consumed);
        assert_eq!(render_state.preedit, "chea");
        assert_eq!(render_state.candidates, vec!["candidate", "chea"]);
        assert_eq!(render_state.selected_index, Some(0));
        assert_eq!(render_state.commit_text.as_deref(), Some("candidate"));
        assert_eq!(render_state.cursor_location.x, 10);
        assert!(render_state.actions.contains(&render::RenderAction::CommitText));
    }

    #[test]
    fn exports_module_boundaries() {
        assert!(input::key_convert::KEY_CONVERSION_IMPLEMENTED);
        assert_eq!(
            session_driver::FIRST_IMPLEMENTATION_MILESTONE,
            "pure Rust Windows session driver around ImeSession"
        );
        assert_eq!(history::PLANNED_HISTORY_PATH, "%APPDATA%\\khmerime\\history.tsv");
    }
}
