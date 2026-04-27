use std::collections::{HashMap, HashSet};

use khmerime_core::{
    build_segmented_session, move_session_focus, normalize_visible_suggestions, normalized_suggestion_key,
    reflow_segmented_session_from_selection, SegmentedSession, Transliterator,
};
use serde::Serialize;

const KEY_BACKSPACE: u32 = 0xFF08;
const KEY_ESCAPE: u32 = 0xFF1B;
const KEY_LEFT: u32 = 0xFF51;
const KEY_UP: u32 = 0xFF52;
const KEY_RIGHT: u32 = 0xFF53;
const KEY_DOWN: u32 = 0xFF54;
const KEY_RETURN: u32 = 0xFF0D;
const KEY_KP_ENTER: u32 = 0xFF8D;
const KEY_SPACE: u32 = 0x20;

const STATE_CONTROL_MASK: u32 = 1 << 2;
const STATE_MOD1_MASK: u32 = 1 << 3;
const STATE_SUPER_MASK: u32 = 1 << 26;
const STATE_HYPER_MASK: u32 = 1 << 27;
const STATE_META_MASK: u32 = 1 << 28;
const STATE_RELEASE_MASK: u32 = 1 << 30;

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
    /// Native platform key code for diagnostics or future platform-specific use.
    pub keycode: u32,
    /// Modifier/release bitmask normalized by the adapter.
    pub state: u32,
}

/// Adapter-facing command model for native IME integrations.
///
/// All platform callbacks should be reduced to this enum before they affect
/// shared IME behavior. This keeps OS-specific lifecycle and key APIs out of
/// the core transliteration engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionCommand {
    ProcessKeyEvent(NativeKeyEvent),
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
    pub preedit: String,
    pub raw_preedit: String,
    pub candidates: Vec<String>,
    pub candidate_display: Vec<CandidateDisplayEntry>,
    pub selected_index: Option<usize>,
    pub segmented_active: bool,
    pub focused_segment_index: Option<usize>,
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

pub struct ImeSession {
    transliterator: Transliterator,
    history: HashMap<String, usize>,
    enabled: bool,
    focused: bool,
    composition_raw: String,
    candidates: Vec<String>,
    selected_index: usize,
    segmented_session: Option<SegmentedSession>,
    cursor_location: CursorLocation,
}

impl ImeSession {
    pub fn new(transliterator: Transliterator, history: HashMap<String, usize>) -> Self {
        Self {
            transliterator,
            history,
            enabled: true,
            focused: false,
            composition_raw: String::new(),
            candidates: Vec::new(),
            selected_index: 0,
            segmented_session: None,
            cursor_location: CursorLocation::default(),
        }
    }

    pub fn from_store<S: HistoryStore>(transliterator: Transliterator, store: &S) -> Result<Self, S::Error> {
        let history = store.load()?;
        Ok(Self::new(transliterator, history))
    }

    pub fn save_history<S: HistoryStore>(&self, store: &S) -> Result<(), S::Error> {
        store.save(&self.history)
    }

    pub fn focus_in(&mut self) {
        self.focused = true;
    }

    pub fn focus_out(&mut self) {
        self.focused = false;
        self.reset();
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.composition_raw.clear();
        self.candidates.clear();
        self.selected_index = 0;
        self.segmented_session = None;
    }

    pub fn set_cursor_location(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.cursor_location = CursorLocation { x, y, width, height };
    }

    pub fn history(&self) -> &HashMap<String, usize> {
        &self.history
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        let segmented_active = self.segmented_session.is_some();
        let preedit = self
            .segmented_session
            .as_ref()
            .map(SegmentedSession::composed_text)
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| self.composition_raw.clone());
        let candidates = self
            .segmented_session
            .as_ref()
            .map(SegmentedSession::focused_candidates)
            .unwrap_or_else(|| self.candidates.clone());
        let selected_index = if candidates.is_empty() {
            None
        } else {
            self.segmented_session
                .as_ref()
                .map(SegmentedSession::focused_selected)
                .or(Some(self.selected_index))
        };
        let candidate_input = self
            .segmented_session
            .as_ref()
            .and_then(|session| session.segments.get(session.focused))
            .map(|segment| segment.input.as_str())
            .unwrap_or(self.composition_raw.as_str());
        let recommended_keys = self
            .transliterator
            .exact_match_targets(candidate_input)
            .into_iter()
            .map(|item| normalized_suggestion_key(&item))
            .collect::<HashSet<_>>();
        let candidate_display = candidates
            .iter()
            .map(|item| {
                let mut roman_hints = self.transliterator.exact_match_roman_variants(candidate_input, item);
                roman_hints.truncate(3);
                CandidateDisplayEntry {
                    output: item.clone(),
                    recommended: recommended_keys.contains(&normalized_suggestion_key(item)),
                    roman_hints,
                }
            })
            .collect::<Vec<_>>();
        let focused_segment_index = self.segmented_session.as_ref().map(|session| session.focused);
        let segment_preview = self
            .segmented_session
            .as_ref()
            .map(|session| {
                session
                    .segments
                    .iter()
                    .enumerate()
                    .map(|(index, segment)| SegmentPreviewEntry {
                        output: segment.selected_text(),
                        input: segment.input.clone(),
                        focused: index == session.focused,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        SessionSnapshot {
            enabled: self.enabled,
            focused: self.focused,
            preedit,
            raw_preedit: self.composition_raw.clone(),
            candidates,
            candidate_display,
            selected_index,
            segmented_active,
            focused_segment_index,
            segment_preview,
            cursor_location: self.cursor_location,
        }
    }

    pub fn process_command(&mut self, command: SessionCommand) -> SessionResult {
        match command {
            SessionCommand::ProcessKeyEvent(event) => self.process_native_key_event(event),
            SessionCommand::FocusIn => {
                self.focus_in();
                SessionResult::default()
            }
            SessionCommand::FocusOut => {
                self.focus_out();
                SessionResult::default()
            }
            SessionCommand::Reset => {
                self.reset();
                SessionResult::default()
            }
            SessionCommand::Enable => {
                self.enable();
                SessionResult::default()
            }
            SessionCommand::Disable => {
                self.disable();
                SessionResult::default()
            }
            SessionCommand::SetCursorLocation(location) => {
                self.set_cursor_location(location.x, location.y, location.width, location.height);
                SessionResult::default()
            }
        }
    }

    pub fn process_native_key_event(&mut self, event: NativeKeyEvent) -> SessionResult {
        self.process_key_event(event.keyval, event.keycode, event.state)
    }

    pub fn process_key_event(&mut self, keyval: u32, _keycode: u32, state: u32) -> SessionResult {
        if !self.enabled {
            return SessionResult::default();
        }
        if !self.focused {
            self.focused = true;
        }

        if is_modifier_only(state) || is_key_release(state) {
            return SessionResult::default();
        }

        match keyval {
            KEY_LEFT => self.handle_left(),
            KEY_UP => self.handle_up(),
            KEY_RIGHT => self.handle_right(),
            KEY_DOWN => self.handle_down(),
            KEY_SPACE => self.handle_space(),
            KEY_RETURN | KEY_KP_ENTER => self.commit_selected_or_raw(),
            KEY_BACKSPACE => self.handle_backspace(),
            KEY_ESCAPE => self.handle_escape(),
            _ => {
                if let Some(index) = keyval_to_digit_index(keyval) {
                    return self.handle_digit(index, keyval);
                }
                if let Some(ch) = keyval_to_ascii_char(keyval) {
                    return self.handle_printable(ch);
                }
                SessionResult::default()
            }
        }
    }

    fn handle_printable(&mut self, ch: char) -> SessionResult {
        let normalized = if ch.is_ascii_alphabetic() {
            ch.to_ascii_lowercase()
        } else {
            ch
        };
        self.composition_raw.push(normalized);
        self.recompute_composition_state();
        if self.should_auto_commit_single_keycap(normalized) {
            let commit_text = self.selected_or_raw_fallback();
            self.reset();
            return SessionResult {
                consumed: true,
                commit_text: Some(commit_text),
                history_changed: false,
            };
        }
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    fn handle_left(&mut self) -> SessionResult {
        let Some(mut session) = self.segmented_session.clone() else {
            return SessionResult::default();
        };
        move_session_focus(&mut session, -1);
        self.segmented_session = Some(session);
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    fn handle_right(&mut self) -> SessionResult {
        let Some(mut session) = self.segmented_session.clone() else {
            return SessionResult::default();
        };
        move_session_focus(&mut session, 1);
        self.segmented_session = Some(session);
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    fn handle_up(&mut self) -> SessionResult {
        self.cycle_candidates(-1)
    }

    fn handle_down(&mut self) -> SessionResult {
        self.cycle_candidates(1)
    }

    fn handle_space(&mut self) -> SessionResult {
        self.cycle_candidates(1)
    }

    fn cycle_candidates(&mut self, delta: isize) -> SessionResult {
        if self.composition_raw.is_empty() {
            return SessionResult::default();
        }

        if let Some(session) = self.segmented_session.clone() {
            let focused = session.focused;
            let Some(segment) = session.segments.get(focused) else {
                return SessionResult::default();
            };
            if segment.candidates.is_empty() {
                return SessionResult {
                    consumed: true,
                    ..SessionResult::default()
                };
            }
            let next_index = offset_index(segment.selected, segment.candidates.len(), delta);
            self.select_focused_segment_candidate(next_index);
            return SessionResult {
                consumed: true,
                ..SessionResult::default()
            };
        }

        if self.candidates.is_empty() {
            return SessionResult::default();
        }

        self.selected_index = offset_index(self.selected_index, self.candidates.len(), delta);
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    fn handle_backspace(&mut self) -> SessionResult {
        if self.composition_raw.is_empty() {
            return SessionResult::default();
        }
        self.composition_raw.pop();
        self.recompute_composition_state();
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    fn handle_escape(&mut self) -> SessionResult {
        if self.composition_raw.is_empty() {
            return SessionResult::default();
        }
        self.reset();
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    fn handle_digit(&mut self, index: usize, keyval: u32) -> SessionResult {
        if self.composition_raw.is_empty() {
            if let Some(ch) = keyval_to_ascii_char(keyval) {
                return self.handle_printable(ch);
            }
            return SessionResult::default();
        }

        if let Some(session) = self.segmented_session.clone() {
            let Some(segment) = session.segments.get(session.focused) else {
                return SessionResult::default();
            };
            if index < segment.candidates.len() {
                self.select_focused_segment_candidate(index);
            }
            return SessionResult {
                consumed: true,
                ..SessionResult::default()
            };
        }

        if self.candidates.is_empty() {
            if let Some(ch) = keyval_to_ascii_char(keyval) {
                return self.handle_printable(ch);
            }
            return SessionResult::default();
        }

        if index < self.candidates.len() {
            self.selected_index = index;
        }
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    fn commit_selected_or_raw(&mut self) -> SessionResult {
        if self.composition_raw.is_empty() {
            return SessionResult::default();
        }

        let commit_text = if let Some(session) = &self.segmented_session {
            let composed = session.composed_text();
            if composed.is_empty() {
                self.selected_or_raw_fallback()
            } else {
                composed
            }
        } else {
            self.selected_or_raw_fallback()
        };
        let history_changed = !commit_text.is_empty() && commit_text != self.composition_raw;
        if history_changed {
            Transliterator::learn(&mut self.history, &commit_text);
        }
        self.reset();
        SessionResult {
            consumed: true,
            commit_text: Some(commit_text),
            history_changed,
        }
    }

    fn selected_or_raw_fallback(&self) -> String {
        self.candidates
            .get(self.selected_index)
            .cloned()
            .unwrap_or_else(|| self.composition_raw.clone())
    }

    fn select_focused_segment_candidate(&mut self, index: usize) {
        let Some(mut session) = self.segmented_session.clone() else {
            return;
        };
        let focused = session.focused;
        let Some(segment) = session.segments.get(focused) else {
            return;
        };
        if index >= segment.candidates.len() {
            return;
        }
        session.segments[focused].selected = index;
        self.segmented_session = Some(self.maybe_reflow_segmented_session(session));
    }

    fn maybe_reflow_segmented_session(&self, session: SegmentedSession) -> SegmentedSession {
        let transliterator = &self.transliterator;
        let suggest = |input: &str, history: &HashMap<String, usize>| -> Vec<String> {
            normalize_visible_suggestions(transliterator.suggest(input, history))
        };
        reflow_segmented_session_from_selection(
            &session,
            &self.history,
            &suggest,
            &|input, target| transliterator.best_prefix_consumption(input, target),
            &|input, history| transliterator.shadow_observation(input, history),
        )
        .unwrap_or(session)
    }

    fn should_auto_commit_single_keycap(&self, typed_char: char) -> bool {
        if !is_single_keycap_char(typed_char) {
            return false;
        }
        if self.composition_raw.chars().count() != 1 {
            return false;
        }
        if self.segmented_session.is_some() {
            return false;
        }
        self.candidates.len() == 1
    }

    fn recompute_composition_state(&mut self) {
        if self.composition_raw.is_empty() {
            self.candidates.clear();
            self.selected_index = 0;
            self.segmented_session = None;
            return;
        }

        self.candidates =
            normalize_visible_suggestions(self.transliterator.suggest(&self.composition_raw, &self.history));
        self.selected_index = 0;

        let observation = self
            .transliterator
            .shadow_observation(&self.composition_raw, &self.history);
        let transliterator = &self.transliterator;
        self.segmented_session =
            build_segmented_session(&observation, &self.composition_raw, &self.history, &|input, history| {
                normalize_visible_suggestions(transliterator.suggest(input, history))
            });
    }
}

fn is_modifier_only(state: u32) -> bool {
    state & (STATE_CONTROL_MASK | STATE_MOD1_MASK | STATE_SUPER_MASK | STATE_HYPER_MASK | STATE_META_MASK) != 0
}

fn is_key_release(state: u32) -> bool {
    state & STATE_RELEASE_MASK != 0
}

fn keyval_to_digit_index(keyval: u32) -> Option<usize> {
    let ch = char::from_u32(keyval)?;
    if !ch.is_ascii_digit() || ch == '0' {
        return None;
    }
    Some((ch as u8 - b'1') as usize)
}

fn keyval_to_ascii_char(keyval: u32) -> Option<char> {
    let ch = char::from_u32(keyval)?;
    if ch.is_ascii_graphic() {
        Some(ch)
    } else {
        None
    }
}

fn offset_index(current: usize, len: usize, delta: isize) -> usize {
    debug_assert!(len > 0);
    (current as isize + delta).rem_euclid(len as isize) as usize
}

fn is_single_keycap_char(ch: char) -> bool {
    matches!(
        ch,
        '0'..='9' | '!' | '"' | '#' | '$' | '%' | '&' | '\'' | '(' | ')' | '~' | '='
    )
}

#[cfg(test)]
mod tests {
    use super::{CursorLocation, ImeSession, NativeKeyEvent, SessionCommand};
    use khmerime_core::{DecoderConfig, Transliterator};
    use std::collections::HashMap;

    fn session() -> ImeSession {
        let fixture = "jea\tជា\nchea\tជា\ntov\tទៅ\nkhnhom\tខ្ញុំ\nkhnhom\tខ្ញំ\nfoo\tអា\nfoo\tអូ\n";
        let transliterator = Transliterator::from_tsv_str_with_config(fixture, DecoderConfig::shadow_interactive())
            .expect("fixture must parse");
        let mut session = ImeSession::new(transliterator, HashMap::new());
        session.focus_in();
        session
    }

    fn type_ascii(session: &mut ImeSession, text: &str) {
        for ch in text.chars() {
            session.process_key_event(ch as u32, 0, 0);
        }
    }

    #[test]
    fn command_surface_accepts_native_key_event() {
        let mut session = session();
        let update = session.process_command(SessionCommand::ProcessKeyEvent(NativeKeyEvent {
            keyval: 'j' as u32,
            keycode: 0,
            state: 0,
        }));
        assert!(update.consumed);
        assert_eq!(session.snapshot().raw_preedit, "j");
    }

    #[test]
    fn printable_ascii_updates_composition() {
        let mut session = session();
        let update = session.process_key_event('j' as u32, 0, 0);
        assert!(update.consumed);
        assert_eq!(session.snapshot().raw_preedit, "j");
    }

    #[test]
    fn space_cycles_candidates() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let snapshot = session.snapshot();
        assert_eq!(snapshot.selected_index, Some(0));
        assert_eq!(snapshot.candidate_display.len(), snapshot.candidates.len());
        assert_eq!(snapshot.candidates.last().map(String::as_str), Some("jea"));
        let update = session.process_key_event(0x20, 0, 0);
        assert!(update.consumed);
        assert_eq!(session.snapshot().selected_index, Some(1));
    }

    #[test]
    fn snapshot_exposes_recommended_and_roman_hint_metadata() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let snapshot = session.snapshot();
        assert!(!snapshot.candidate_display.is_empty());

        let recommended = snapshot
            .candidate_display
            .iter()
            .filter(|entry| entry.recommended)
            .collect::<Vec<_>>();
        assert!(!recommended.is_empty());
        assert!(recommended
            .iter()
            .any(|entry| entry.roman_hints.iter().any(|hint| hint == "jea")));
    }

    #[test]
    fn enter_commits_selected_candidate() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let update = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(update.commit_text.as_deref(), Some("ជា"));
        assert!(update.history_changed);
        assert!(session.snapshot().preedit.is_empty());
    }

    #[test]
    fn digit_selects_candidate_without_immediate_commit() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let update = session.process_key_event('1' as u32, 0, 0);
        assert!(update.consumed);
        assert!(update.commit_text.is_none());
        let committed = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(committed.commit_text.as_deref(), Some("ជា"));
    }

    #[test]
    fn single_digit_keycap_commits_immediately() {
        let mut session = session();
        let update = session.process_key_event('1' as u32, 0, 0);
        assert!(update.consumed);
        assert_eq!(update.commit_text.as_deref(), Some("១"));
        assert!(!update.history_changed);
        assert!(session.snapshot().preedit.is_empty());
    }

    #[test]
    fn single_symbol_keycap_commits_immediately() {
        let mut session = session();
        let update = session.process_key_event('=' as u32, 0, 0);
        assert!(update.consumed);
        assert_eq!(update.commit_text.as_deref(), Some("៌"));
        assert!(!update.history_changed);
        assert!(session.snapshot().preedit.is_empty());
    }

    #[test]
    fn selecting_raw_fallback_candidate_commits_literal_without_learning() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let candidate_len = session.snapshot().candidates.len();
        assert!(candidate_len >= 2);

        for _ in 1..candidate_len {
            let down = session.process_key_event(0xFF54, 0, 0);
            assert!(down.consumed);
        }

        let snapshot = session.snapshot();
        assert_eq!(snapshot.selected_index, Some(candidate_len - 1));
        assert_eq!(snapshot.candidates.last().map(String::as_str), Some("jea"));

        let update = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(update.commit_text.as_deref(), Some("jea"));
        assert!(!update.history_changed);
    }

    #[test]
    fn segment_focus_moves_with_left_right() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let snapshot = session.snapshot();
        assert!(snapshot.segmented_active);
        assert_eq!(snapshot.focused_segment_index, Some(0));
        assert!(!snapshot.segment_preview.is_empty());

        let right = session.process_key_event(0xFF53, 0, 0);
        assert!(right.consumed);
        assert_eq!(session.snapshot().focused_segment_index, Some(1));

        let left = session.process_key_event(0xFF51, 0, 0);
        assert!(left.consumed);
        assert_eq!(session.snapshot().focused_segment_index, Some(0));
    }

    #[test]
    fn up_down_cycle_segment_candidates_without_moving_focus() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");

        let snapshot = session.snapshot();
        assert!(snapshot.segmented_active);
        assert_eq!(snapshot.focused_segment_index, Some(0));
        assert_eq!(snapshot.selected_index, Some(0));
        assert!(snapshot.candidates.len() >= 2);

        let down = session.process_key_event(0xFF54, 0, 0);
        assert!(down.consumed);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.focused_segment_index, Some(0));
        assert_eq!(snapshot.selected_index, Some(1));

        let up = session.process_key_event(0xFF52, 0, 0);
        assert!(up.consumed);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.focused_segment_index, Some(0));
        assert_eq!(snapshot.selected_index, Some(0));
    }

    #[test]
    fn enter_commits_full_segmented_phrase() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let update = session.process_key_event(0xFF0D, 0, 0);
        let commit_text = update.commit_text.expect("must commit text");
        assert!(!commit_text.is_empty());
        assert_ne!(commit_text, "khnhomtov");
    }

    #[test]
    fn left_right_pass_through_without_segmented_session() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let left = session.process_key_event(0xFF51, 0, 0);
        let right = session.process_key_event(0xFF53, 0, 0);
        assert!(!left.consumed);
        assert!(!right.consumed);
    }

    #[test]
    fn up_down_cycle_flat_candidates() {
        let mut session = session();
        type_ascii(&mut session, "foo");
        let snapshot = session.snapshot();
        assert!(!snapshot.segmented_active);
        assert_eq!(snapshot.selected_index, Some(0));
        assert!(snapshot.candidates.len() >= 2);

        let down = session.process_key_event(0xFF54, 0, 0);
        assert!(down.consumed);
        assert_eq!(session.snapshot().selected_index, Some(1));

        let up = session.process_key_event(0xFF52, 0, 0);
        assert!(up.consumed);
        assert_eq!(session.snapshot().selected_index, Some(0));
    }

    #[test]
    fn up_down_pass_through_without_active_candidate_ui() {
        let mut session = session();
        let snapshot = session.snapshot();
        assert!(snapshot.candidates.is_empty());
        assert!(!snapshot.segmented_active);

        let down = session.process_key_event(0xFF54, 0, 0);
        let up = session.process_key_event(0xFF52, 0, 0);
        assert!(!down.consumed);
        assert!(!up.consumed);
    }

    #[test]
    fn backspace_edits_composition() {
        let mut session = session();
        type_ascii(&mut session, "je");
        let update = session.process_key_event(0xFF08, 0, 0);
        assert!(update.consumed);
        assert_eq!(session.snapshot().raw_preedit, "j");
    }

    #[test]
    fn escape_cancels_composition() {
        let mut session = session();
        session.process_key_event('j' as u32, 0, 0);
        let update = session.process_key_event(0xFF1B, 0, 0);
        assert!(update.consumed);
        assert!(session.snapshot().preedit.is_empty());
        assert!(session.snapshot().segment_preview.is_empty());
    }

    #[test]
    fn enter_with_no_match_commits_raw_roman() {
        let mut session = session();
        for key in ['x', 'x', 'x'] {
            session.process_key_event(key as u32, 0, 0);
        }
        let update = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(update.commit_text.as_deref(), Some("xxx"));
        assert!(!update.history_changed);
    }

    #[test]
    fn set_cursor_location_updates_snapshot() {
        let mut session = session();
        session.set_cursor_location(1, 2, 3, 4);
        assert_eq!(
            session.snapshot().cursor_location,
            CursorLocation {
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            }
        );
    }

    #[test]
    fn control_modified_key_is_not_consumed() {
        let mut session = session();
        let update = session.process_key_event('a' as u32, 0, 1 << 2);
        assert!(!update.consumed);
        assert!(session.snapshot().preedit.is_empty());
    }

    #[test]
    fn focus_out_resets_preedit() {
        let mut session = session();
        session.process_key_event('j' as u32, 0, 0);
        session.focus_out();
        assert!(session.snapshot().preedit.is_empty());
    }
}
