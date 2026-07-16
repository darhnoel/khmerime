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
/// `Roman` is the existing decoder-backed KhmerIME flow. `Nida` is direct Khmer
/// keymap input — mapped printable keys commit immediately with no Composition.
/// `CharPick` is phonetic character lookup — each roman keystroke returns all
/// Khmer characters whose relation includes that letter as candidates; tapping
/// one commits it immediately with no preedit accumulation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    #[default]
    Roman,
    Nida,
    CharPick,
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
    /// Select Phrase Candidate `i` from the wheel as the active Segmented Session
    /// (ADR-0014), so the next commit takes that whole-phrase hypothesis.
    SelectPhrase(usize),
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
    /// Ranked whole-composition Khmer hypotheses — the **Phrase Candidate** list the
    /// mobile Phrase Wheel scrolls (ADR-0014).
    pub phrase_candidates: Vec<PhraseCandidate>,
    /// Index into `phrase_candidates` whose phrase currently drives the strip preview.
    /// Defaults to `0`; selecting another phrase updates this so renderers can show
    /// every candidate except the one already previewed in the strip.
    pub selected_phrase_index: usize,
    pub cursor_location: CursorLocation,
}

/// One whole-composition Khmer hypothesis — a **Phrase Candidate** (ADR-0014).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PhraseCandidate {
    /// The complete Khmer rendering of the entire composition.
    pub text: String,
    /// This hypothesis's own segmentation (one entry per word), for the wheel card's
    /// Roman Row and for Level-2 editing. Its outputs concatenate to `text`.
    pub segments: Vec<PhraseSegment>,
    /// True when the model provider contributed to this phrase — for the UI's model-assisted
    /// marker.
    pub from_model: bool,
    /// True when every word in this hypothesis is present in the Lexicon.
    pub lexicon_verified: bool,
}

/// One word inside a **Phrase Candidate**: its roman slice and Khmer output.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PhraseSegment {
    pub input: String,
    pub output: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CandidateDisplayEntry {
    /// Candidate text to render.
    pub output: String,
    /// Whether ranking marks this candidate as the recommended/default choice.
    pub recommended: bool,
    /// Roman hints that explain why this candidate matched the current input.
    pub roman_hints: Vec<String>,
    /// True when this candidate is the raw roman string kept as the **Commit
    /// Rules** floor. Renderers that hide ASCII candidates must still show this
    /// one, so the user can always fall back to committing their literal input.
    pub is_raw_fallback: bool,
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

/// Default number of candidate rows a host paints per page. Adapters that
/// paginate the **Candidate List** (IBus) override this; adapters that keep a
/// single page or use a native candidate window leave it at the default, which
/// keeps digit selection and `0`-row handling behaving exactly as before.
pub const DEFAULT_PAGE_SIZE: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImeSessionOptions {
    pub segmented_preview: SegmentedPreviewMode,
    /// How many candidates the host displays per page. Selection math
    /// (page-relative digits, the `0`-row gate) is derived from this, so it
    /// must equal what the adapter actually paints.
    pub page_size: usize,
}

impl Default for ImeSessionOptions {
    fn default() -> Self {
        Self {
            segmented_preview: SegmentedPreviewMode::default(),
            page_size: DEFAULT_PAGE_SIZE,
        }
    }
}
