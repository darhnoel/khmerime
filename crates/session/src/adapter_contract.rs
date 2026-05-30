//! Adapter-facing boundary types for native IME integrations.
//!
//! These are the types platform adapters (IBus, TSF, Dioxus) translate their
//! callbacks into and render from. They intentionally hold no engine internals
//! or platform widget handles, keeping OS-specific concerns out of the shared
//! [`ImeSession`](crate::ime_session::ImeSession) behavior.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Maximum candidate key length desktop history stores should reload.
///
/// Longer persisted keys are treated as legacy pollution from concatenated
/// phrase commits rather than useful learned unigrams.
pub const MAX_PERSISTED_HISTORY_WORD_CHARS: usize = 18;

pub fn should_persist_history_word(word: &str) -> bool {
    word.chars().count() <= MAX_PERSISTED_HISTORY_WORD_CHARS
}

/// Persistence boundary for learned candidate usage.
///
/// Implementations should store the map as simple word/candidate keys to usage
/// counts. The desktop adapters currently use TSV so Khmer text and roman keys
/// do not require CSV quoting.
pub trait HistoryStore {
    type Error;

    fn load(&self) -> Result<HashMap<String, usize>, Self::Error>;
    fn save(&self, history: &HashMap<String, usize>) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CursorLocation {
    /// Screen-space x coordinate used by adapters to anchor candidate UI.
    pub x: i32,
    /// Screen-space y coordinate used by adapters to anchor candidate UI.
    pub y: i32,
    /// Caret or composition rectangle width, when the platform provides it.
    pub width: i32,
    /// Caret or composition rectangle height, when the platform provides it.
    pub height: i32,
}

/// Platform-neutral key payload accepted by `ImeSession`.
///
/// `keyval` follows the current XKB-style contract used by the session for
/// printable Unicode scalars and special keys. Platform adapters must translate
/// native key events into this representation before calling the session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NativeKeyEvent {
    /// Printable Unicode scalar or one of the session's special key constants.
    pub keyval: u32,
    /// Canonical scancode for the pressed physical key: the set-1 / evdev
    /// numbering of the main typing block (e.g. `Q` = 16, `1` = 2, Space = 57).
    ///
    /// Adapters must normalize their native key codes into this space. Linux
    /// IBus already delivers evdev keycodes that match; the Windows TSF adapter
    /// forwards its `lParam` scan code. NIDA-mode keymap lookup depends on this;
    /// pass `0` when no physical key is available and the session will fall back
    /// to `keyval`.
    pub keycode: u32,
    /// Modifier/release bitmask normalized by the adapter.
    pub state: u32,
}

/// Shared input mode for native IME sessions.
///
/// `Roman` is the existing decoder-backed KhmerIME flow. `Nida` is reserved for
/// direct Khmer keymap input, where mapped printable keys commit immediately and
/// decoder composition stays inactive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    #[default]
    Roman,
    Nida,
}

/// Adapter-facing command model for native IME integrations.
///
/// All platform callbacks should be reduced to this enum before they affect
/// shared IME behavior. This keeps OS-specific lifecycle and key APIs out of
/// the core transliteration engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionCommand {
    ProcessKeyEvent(NativeKeyEvent),
    SetInputMode(InputMode),
    ToggleInputMode,
    FocusIn,
    FocusOut,
    Reset,
    Enable,
    Disable,
    SetCursorLocation(CursorLocation),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SegmentPreviewEntry {
    /// Khmer output for this segment using the current selected candidate.
    pub output: String,
    /// Roman input range represented by the segment.
    pub input: String,
    /// Whether this segment currently owns candidate navigation focus.
    pub focused: bool,
}

/// Render-facing snapshot of the current IME state.
///
/// Adapters should treat this as the single source of truth for preedit,
/// candidate list, segment preview, selected candidate, and cursor anchoring.
/// It intentionally contains no platform widget handles.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SessionSnapshot {
    pub enabled: bool,
    pub focused: bool,
    pub input_mode: InputMode,
    pub preedit: String,
    pub raw_preedit: String,
    pub candidates: Vec<String>,
    pub candidate_display: Vec<CandidateDisplayEntry>,
    pub selected_index: Option<usize>,
    pub segmented_active: bool,
    pub focused_segment_index: Option<usize>,
    pub segment_edit_active: bool,
    pub segment_edit_index: Option<usize>,
    pub segment_preview: Vec<SegmentPreviewEntry>,
    pub cursor_location: CursorLocation,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CandidateDisplayEntry {
    /// Candidate text to render.
    pub output: String,
    /// Whether ranking marks this candidate as the recommended/default choice.
    pub recommended: bool,
    /// Roman hints that explain why this candidate matched the current input.
    pub roman_hints: Vec<String>,
}

/// Result of processing one adapter command.
///
/// `consumed` controls whether the host application should also receive the
/// original key. `commit_text` is one-shot: adapters must commit it once and then
/// rely on the next snapshot for display state. `history_changed` tells adapters
/// when learned usage should be persisted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionResult {
    pub consumed: bool,
    pub commit_text: Option<String>,
    pub history_changed: bool,
}

pub type ImeSessionSnapshot = SessionSnapshot;
pub type ImeSessionUpdate = SessionResult;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SegmentedPreviewMode {
    Disabled,
    Deferred,
    #[default]
    Enabled,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImeSessionOptions {
    pub segmented_preview: SegmentedPreviewMode,
}
