//! iOS custom-keyboard adapter scaffold.
//!
//! This crate is intentionally a **commented contract skeleton**.
//! It documents how an iOS keyboard extension (`UIInputViewController`)
//! should translate platform callbacks into shared `khmerime_session`
//! commands without embedding Dioxus runtime code.
//!
//! References:
//! - UIInputViewController: <https://developer.apple.com/documentation/uikit/uiinputviewcontroller>
//! - UITextDocumentProxy: <https://developer.apple.com/documentation/uikit/uitextdocumentproxy>
//! - Custom Keyboard Guide:
//!   <https://developer.apple.com/library/archive/documentation/General/Conceptual/ExtensibilityPG/CustomKeyboard.html>

use khmerime_session::{CursorLocation, NativeKeyEvent, SessionCommand, SessionResult, SessionSnapshot};

/// Native iOS keyboard-extension callbacks that eventually need wiring.
///
/// These are *logical* callback names. Real implementation will be in Swift/Obj-C,
/// then forwarded into Rust adapter code by a thin FFI boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IosKeyboardCallback {
    /// Keyboard view became active for a host text field.
    ViewDidAppear,
    /// Keyboard view is being dismissed.
    ViewWillDisappear,
    /// Host text/selection changed. Useful to refresh context-dependent UI.
    TextDidChange,
    /// Host cursor moved/selection changed.
    SelectionDidChange,
    /// Hardware/software key tap routed from native layer.
    KeyInput(NativeKeyEvent),
    /// Keyboard's own cursor/caret anchor (screen-space) changed.
    CursorLocationChanged(CursorLocation),
    /// Explicit user cancel action from keyboard chrome.
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
        callback: "viewDidAppear",
        session_intent: "focus_in",
        notes: "start IME session when keyboard extension becomes visible",
    },
    CallbackMapping {
        callback: "viewWillDisappear",
        session_intent: "focus_out",
        notes: "drop preedit state when keyboard leaves focus",
    },
    CallbackMapping {
        callback: "textDidChange",
        session_intent: "snapshot refresh",
        notes: "host text context changed; recompute keyboard UI only",
    },
    CallbackMapping {
        callback: "selectionDidChange",
        session_intent: "set_cursor_location",
        notes: "sync segment preview anchor with host cursor",
    },
    CallbackMapping {
        callback: "keyInput",
        session_intent: "process_key_event",
        notes: "main transliteration key path",
    },
];

/// Minimal render responsibilities from a session snapshot/result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IosRenderState {
    /// Candidate rows shown in the keyboard's candidate strip.
    pub candidates: Vec<String>,
    /// Preedit/composition text shown in keyboard-owned UI.
    pub preedit: String,
    /// Optional commit text to push through `textDocumentProxy.insertText`.
    pub commit_text: Option<String>,
}

/// Returns a static callback-to-command map for contributor onboarding and tests.
pub fn callback_map() -> &'static [CallbackMapping] {
    CALLBACK_MAP
}

/// Converts iOS callback intent to a session command.
///
/// Implemented later once Swift callback/FFI wiring lands.
pub fn map_callback_to_session_command(_callback: &IosKeyboardCallback) -> Option<SessionCommand> {
    // Implemented later:
    // - Map visibility callbacks to FocusIn/FocusOut.
    // - Convert native key payloads to SessionCommand::ProcessKeyEvent.
    // - Forward cursor geometry to SetCursorLocation when available.
    None
}

/// Derives UI update instructions from session output.
///
/// iOS custom keyboards own their candidate/preedit UI, so this render struct is
/// intentionally separate from any Dioxus web/desktop components.
pub fn derive_render_state(snapshot: &SessionSnapshot, result: &SessionResult) -> IosRenderState {
    IosRenderState {
        candidates: snapshot.candidates.clone(),
        preedit: snapshot.preedit.clone(),
        commit_text: result.commit_text.clone(),
    }
}
