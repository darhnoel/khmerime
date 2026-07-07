//! iOS custom-keyboard adapter.
//!
//! `KhmerIMESession` wraps `khmerime_session::ImeSession` behind a UniFFI-exported
//! Arc<Mutex<_>> handle. Swift calls one method per key tap; each returns a fresh
//! `IosRenderState` that drives the strip + candidate panel.

use std::sync::{Arc, Mutex, OnceLock};

use khmerime_core::{DecoderConfig, SharedTransliteratorData, Transliterator};
use khmerime_session::{
    ImeSession, ImeSessionOptions, NativeKeyEvent, PhraseCandidate, PhraseSegment, SegmentPreviewEntry,
    SegmentedPreviewMode, SessionResult, SessionSnapshot,
};

uniffi::setup_scaffolding!("khmerime_ios_keyboard");

// ── Key constants ────────────────────────────────────────────────────────────

const KEY_BACKSPACE: u32 = 0xFF08;
const KEY_RETURN: u32 = 0xFF0D;
const KEY_SPACE: u32 = 0x20;
const KEY_LEFT: u32 = 0xFF51;
const KEY_RIGHT: u32 = 0xFF53;
const KEY_TAB: u32 = 0xFF09;

fn key_event(keyval: u32) -> NativeKeyEvent {
    NativeKeyEvent {
        keyval,
        keycode: 0,
        state: 0,
    }
}

static SHARED_TRANSLITERATOR_DATA: OnceLock<SharedTransliteratorData> = OnceLock::new();

fn shared_transliterator_data() -> &'static SharedTransliteratorData {
    SHARED_TRANSLITERATOR_DATA
        .get_or_init(|| Transliterator::from_default_shared_data().expect("compiled-in lexicon data must be valid"))
}

// ── Public UniFFI types ───────────────────────────────────────────────────────

/// One entry in the expanded segment panel / phrase bar.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct IosSegmentEntry {
    pub output: String,
    pub input: String,
    pub focused: bool,
}

impl From<&SegmentPreviewEntry> for IosSegmentEntry {
    fn from(s: &SegmentPreviewEntry) -> Self {
        IosSegmentEntry {
            output: s.output.clone(),
            input: s.input.clone(),
            focused: s.focused,
        }
    }
}

impl From<&PhraseSegment> for IosSegmentEntry {
    fn from(s: &PhraseSegment) -> Self {
        IosSegmentEntry {
            output: s.output.clone(),
            input: s.input.clone(),
            focused: false,
        }
    }
}

/// One card of the Phrase Wheel (ADR-0014): a whole-composition Khmer hypothesis
/// with its own segmentation.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct IosPhraseCandidate {
    pub text: String,
    pub segments: Vec<IosSegmentEntry>,
}

impl From<&PhraseCandidate> for IosPhraseCandidate {
    fn from(c: &PhraseCandidate) -> Self {
        IosPhraseCandidate {
            text: c.text.clone(),
            segments: c.segments.iter().map(IosSegmentEntry::from).collect(),
        }
    }
}

/// Render state returned to Swift after every session call.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct IosRenderState {
    /// Khmer candidates for the focused segment.
    pub candidates: Vec<String>,
    /// Currently selected candidate index, if any.
    pub selected_index: Option<u64>,
    /// Raw roman preedit (what the user typed).
    pub preedit: String,
    /// Segment entries for the phrase bar (empty when no segmented session).
    pub segments: Vec<IosSegmentEntry>,
    /// Which segment owns candidate focus.
    pub focused_segment_index: Option<u64>,
    /// Non-None only immediately after Enter; Swift deletes roman buffer and
    /// inserts this concatenated Khmer string.
    pub commit_text: Option<String>,
    /// True while the user is retyping one segment's roman input in isolation.
    pub segment_edit_active: bool,
    /// Which segment is currently being edited (when segment_edit_active).
    pub segment_edit_index: Option<u64>,
    /// Ranked whole-composition hypotheses for the Phrase Wheel (ADR-0014); `[0]` is
    /// the current best, the last entry is the raw roman fallback.
    pub phrase_candidates: Vec<IosPhraseCandidate>,
}

fn render_state(snapshot: &SessionSnapshot, result: &SessionResult) -> IosRenderState {
    IosRenderState {
        candidates: snapshot.candidates.clone(),
        selected_index: snapshot.selected_index.map(|i| i as u64),
        preedit: snapshot.preedit.clone(),
        segments: snapshot.segment_preview.iter().map(IosSegmentEntry::from).collect(),
        focused_segment_index: snapshot.focused_segment_index.map(|i| i as u64),
        commit_text: result.commit_text.clone(),
        segment_edit_active: snapshot.segment_edit_active,
        segment_edit_index: snapshot.segment_edit_index.map(|i| i as u64),
        phrase_candidates: snapshot.phrase_candidates.iter().map(IosPhraseCandidate::from).collect(),
    }
}

// ── Session handle ────────────────────────────────────────────────────────────

/// Swift-visible session handle, exported via UniFFI.
///
/// Wraps the real `ImeSession` so every key tap goes through the full
/// romanization → segmentation → candidate ranking pipeline.
#[derive(uniffi::Object)]
pub struct KhmerIMESession {
    inner: Mutex<ImeSession>,
}

#[uniffi::export]
impl KhmerIMESession {
    /// Called once in `viewDidLoad`. Builds the transliterator from compiled-in
    /// data (no external files needed on iOS).
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        let transliterator = Transliterator::from_shared_data_with_config(
            shared_transliterator_data(),
            DecoderConfig::shadow_interactive(),
        );
        let session = ImeSession::builder(transliterator, std::collections::HashMap::new())
            .input_mode(khmerime_session::InputMode::Roman)
            .options(ImeSessionOptions {
                segmented_preview: SegmentedPreviewMode::Enabled,
                ..Default::default()
            })
            .build();
        Arc::new(KhmerIMESession {
            inner: Mutex::new(session),
        })
    }

    pub fn focus_in(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        s.focus_in();
        render_state(&s.snapshot(), &SessionResult::default())
    }

    pub fn focus_out(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        s.focus_out();
        render_state(&s.snapshot(), &SessionResult::default())
    }

    pub fn process_character(&self, ch: String) -> IosRenderState {
        let keyval = ch.chars().next().unwrap_or('?') as u32;
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(keyval));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_backspace(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_BACKSPACE));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_space(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_SPACE));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_enter(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_RETURN));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_left(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_LEFT));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_right(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_RIGHT));
        render_state(&s.snapshot(), &result)
    }

    /// Toggle Segment Edit Mode on the focused segment.
    /// First call enters edit mode (next printable replaces segment roman input).
    /// Second call exits edit mode.
    pub fn process_tab(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_TAB));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_digit(&self, n: u8) -> IosRenderState {
        let keyval = b'0' as u32 + n as u32;
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(keyval));
        render_state(&s.snapshot(), &result)
    }

    pub fn set_cursor_location(&self, x: i32, y: i32, width: i32, height: i32) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        s.set_cursor_location(x, y, width, height);
        render_state(&s.snapshot(), &SessionResult::default())
    }

    pub fn enter_char_pick(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        s.process_command(khmerime_session::SessionCommand::SetInputMode(
            khmerime_session::InputMode::CharPick,
        ));
        render_state(&s.snapshot(), &SessionResult::default())
    }

    pub fn exit_char_pick(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        s.process_command(khmerime_session::SessionCommand::SetInputMode(
            khmerime_session::InputMode::Roman,
        ));
        render_state(&s.snapshot(), &SessionResult::default())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
//
// All tests go through `KhmerIMESession::new()` (full compiled-in lexicon,
// `DecoderConfig::shadow_interactive()`, `SegmentedPreviewMode::Enabled`) so
// they exercise the exact same code path as the Swift extension.
//
// Key fixture facts:
//   "nhom"       — single word, no segmented session, preedit = "nhom"
//   "khnhomtov"  — two-word phrase, splits into ≥2 segments in shadow mode
//   digit outside composition → auto-commits as Khmer numeral (build_segmented
//     session returns None for a single digit, so should_auto_commit fires)

#[cfg(test)]
mod tests {
    use super::*;

    fn new_session() -> Arc<KhmerIMESession> {
        KhmerIMESession::new()
    }

    fn type_str(s: &Arc<KhmerIMESession>, input: &str) -> IosRenderState {
        let mut state = IosRenderState::default();
        for ch in input.chars() {
            state = s.process_character(ch.to_string());
        }
        state
    }

    // ── render_state field mapping ────────────────────────────────────────────

    #[test]
    fn focus_in_returns_clean_state() {
        let s = new_session();
        let state = s.focus_in();
        assert_eq!(state.preedit, "");
        assert!(state.candidates.is_empty());
        assert!(state.segments.is_empty());
        assert!(state.commit_text.is_none());
        assert!(!state.segment_edit_active);
        assert!(state.segment_edit_index.is_none());
        assert!(state.focused_segment_index.is_none());
        assert!(state.selected_index.is_none());
    }

    // ── phrase wheel (ADR-0014) ───────────────────────────────────────────────

    #[test]
    fn render_state_exposes_phrase_candidates_for_the_wheel() {
        let s = new_session();
        s.focus_in();
        let state = type_str(&s, "khnhomtov");
        assert!(
            !state.phrase_candidates.is_empty(),
            "phrase candidates must cross the FFI so the Phrase Wheel can render"
        );
        let top = &state.phrase_candidates[0];
        assert!(!top.text.is_empty(), "top wheel card must have Khmer text");
        assert!(
            !top.segments.is_empty(),
            "top card must carry its segmentation for the Roman Row / Level-2 editing"
        );
        assert_eq!(
            state.phrase_candidates.last().map(|entry| entry.text.as_str()),
            Some("khnhomtov"),
            "raw roman must be the last wheel card"
        );
    }

    // ── basic composition ─────────────────────────────────────────────────────

    #[test]
    fn letter_input_builds_preedit_and_produces_candidates() {
        let s = new_session();
        s.focus_in();
        let state = type_str(&s, "nhom");
        // "nhom" is a single word; no segmented session → preedit == raw roman input.
        assert_eq!(state.preedit, "nhom");
        assert!(!state.candidates.is_empty(), "nhom must produce at least one candidate");
        assert!(state.commit_text.is_none());
    }

    #[test]
    fn backspace_removes_last_raw_preedit_char() {
        let s = new_session();
        s.focus_in();
        type_str(&s, "nhom");
        // "nhom" stays in the flat session, so preedit == composition_raw.
        // After one backspace the raw input shrinks to "nho".
        let state = s.process_backspace();
        assert_eq!(state.preedit, "nho");
        assert!(state.commit_text.is_none());
    }

    #[test]
    fn enter_commits_composition_to_khmer_text() {
        let s = new_session();
        s.focus_in();
        type_str(&s, "nhom");
        let state = s.process_enter();
        let committed = state.commit_text.expect("enter must produce commit_text");
        assert!(!committed.is_empty());
        // All committed characters should be in the Khmer Unicode block (U+1780–U+17FF).
        assert!(
            committed.chars().all(|c| ('\u{1780}'..='\u{17FF}').contains(&c)),
            "committed text must be Khmer Unicode; got {committed:?}"
        );
    }

    // ── digit key behaviour ───────────────────────────────────────────────────

    #[test]
    fn digit_1_outside_composition_commits_khmer_numeral() {
        let s = new_session();
        s.focus_in();
        // '1' → Khmer numeral ១ (U+17E1). The commit fires because
        // "1" is a single keycap with exactly one candidate (the Khmer digit).
        let state = s.process_digit(1);
        assert_eq!(
            state.commit_text.as_deref(),
            Some("១"),
            "digit 1 outside composition must commit Khmer numeral ១"
        );
        assert!(state.preedit.is_empty());
    }

    #[test]
    fn digit_during_composition_does_not_commit() {
        let s = new_session();
        s.focus_in();
        type_str(&s, "nhom");
        // During composition, digit 1 selects the first candidate; it must not
        // auto-commit (that would lose the user's in-progress word).
        let state = s.process_digit(1);
        assert!(
            state.commit_text.is_none(),
            "digit during composition must not commit immediately"
        );
        assert_eq!(
            state.selected_index,
            Some(0u64),
            "digit 1 during composition must keep selected_index = 0"
        );
    }

    // ── segmentation (requires shadow_interactive + SegmentedPreviewMode::Enabled) ──

    #[test]
    fn two_word_input_produces_multiple_segments() {
        let s = new_session();
        s.focus_in();
        // "khnhomtov" → "khnhom" (ខ្ញុំ) + "tov" (ទៅ). The shadow decoder must
        // split this into ≥2 segments; failure here means shadow_interactive is inactive.
        let state = type_str(&s, "khnhomtov");
        assert!(
            state.segments.len() >= 2,
            "khnhomtov must produce ≥2 segments (got {}); \
             check DecoderConfig::shadow_interactive() is active",
            state.segments.len()
        );
    }

    #[test]
    fn each_segment_has_non_empty_input_and_output() {
        let s = new_session();
        s.focus_in();
        let state = type_str(&s, "khnhomtov");
        assert!(!state.segments.is_empty());
        for (i, seg) in state.segments.iter().enumerate() {
            assert!(!seg.input.is_empty(), "segment {i} input must not be empty");
            assert!(!seg.output.is_empty(), "segment {i} output must not be empty");
        }
    }

    // ── segment edit mode ─────────────────────────────────────────────────────

    #[test]
    fn process_tab_enters_segment_edit_mode_on_segment_0() {
        let s = new_session();
        s.focus_in();
        type_str(&s, "khnhomtov");
        let state = s.process_tab();
        assert!(
            state.segment_edit_active,
            "process_tab must set segment_edit_active = true"
        );
        assert_eq!(
            state.segment_edit_index,
            Some(0u64),
            "edit must begin on focused segment 0"
        );
    }

    #[test]
    fn segment_edit_index_equals_focused_segment_after_tab() {
        let s = new_session();
        s.focus_in();
        type_str(&s, "khnhomtov");
        // Move focus to segment 1, then enter edit mode; both indices must agree.
        s.process_right();
        let state = s.process_tab();
        assert!(state.segment_edit_active);
        assert_eq!(
            state.segment_edit_index, state.focused_segment_index,
            "segment_edit_index must match focused_segment_index on tab entry"
        );
    }

    #[test]
    fn second_tab_exits_segment_edit_mode() {
        let s = new_session();
        s.focus_in();
        type_str(&s, "khnhomtov");
        s.process_tab();
        let state = s.process_tab();
        assert!(
            !state.segment_edit_active,
            "second process_tab must exit segment edit mode"
        );
        assert!(state.segment_edit_index.is_none());
    }

    // ── CharPick mode ─────────────────────────────────────────────────────────

    #[test]
    fn enter_char_pick_switches_mode_and_clears_composition() {
        let s = new_session();
        type_str(&s, "nhom");

        let state = s.enter_char_pick();

        assert!(state.preedit.is_empty(), "preedit must clear on entering CharPick");
        assert!(state.candidates.is_empty(), "no candidates until a key is typed");
    }

    #[test]
    fn process_character_in_char_pick_mode_returns_relation_candidates() {
        let s = new_session();
        s.enter_char_pick();

        let state = s.process_character("k".to_string());

        assert!(
            state.candidates.iter().any(|c| c == "ក"),
            "candidates must include ក for 'k', got: {:?}",
            state.candidates
        );
        assert!(
            state.candidates.iter().any(|c| c == "្ក"),
            "candidates must include ្ក for 'k', got: {:?}",
            state.candidates
        );
        assert!(state.preedit.is_empty(), "preedit must stay empty in CharPick mode");
        assert!(
            state.commit_text.is_none(),
            "process_character must not commit in CharPick mode"
        );
    }

    #[test]
    fn exit_char_pick_returns_to_roman_mode() {
        let s = new_session();
        s.enter_char_pick();

        s.exit_char_pick();
        let state = s.process_character("k".to_string());

        // Back in Roman mode: 'k' builds a composition, not a char-pick lookup
        assert_eq!(state.preedit, "k", "roman mode preedit must be the typed roman letter");
    }
}
