//! The `ImeSession` itself: composition state, constructors, lifecycle
//! (focus/enable/reset/input-mode), and the key-event dispatch that routes each
//! key into the focused behavior module.
//!
//! This module owns the struct and the flat key handlers (printable, NIDA,
//! up/down/space, backspace, escape, digit). The heavier behaviors live in
//! siblings that add their own `impl ImeSession` blocks:
//! [`crate::commit_rules`], [`crate::segmented_session`],
//! [`crate::segment_edit_mode`], and [`crate::session_snapshot`]. Boundary types
//! are in [`crate::adapter_contract`].

use std::collections::HashSet;
use std::sync::Arc;

use khmerime_core::Transliterator;

use std::collections::HashMap;

use crate::adapter_contract::{
    CursorLocation, ImeSessionOptions, InputMode, NativeKeyEvent, PhraseCandidate, SegmentedPreviewMode,
    SessionCommand, SessionResult,
};
use crate::segment_edit_mode::SegmentEditState;
use crate::segment_model::{normalized_suggestion_key, SegmentedSession};

const KEY_BACKSPACE: u32 = 0xFF08;
const KEY_TAB: u32 = 0xFF09;
const KEY_ESCAPE: u32 = 0xFF1B;
const KEY_LEFT: u32 = 0xFF51;
const KEY_UP: u32 = 0xFF52;
const KEY_RIGHT: u32 = 0xFF53;
const KEY_DOWN: u32 = 0xFF54;
const KEY_RETURN: u32 = 0xFF0D;
const KEY_KP_ENTER: u32 = 0xFF8D;
const KEY_SPACE: u32 = 0x20;
const KEY_CAPS_LOCK: u32 = 0xFFE5;

pub(crate) const STATE_SHIFT_MASK: u32 = 1;
const STATE_CONTROL_MASK: u32 = 1 << 2;
const STATE_MOD1_MASK: u32 = 1 << 3;
pub(crate) const STATE_MOD5_MASK: u32 = 1 << 7;
const STATE_SUPER_MASK: u32 = 1 << 26;
const STATE_HYPER_MASK: u32 = 1 << 27;
const STATE_META_MASK: u32 = 1 << 28;
const STATE_RELEASE_MASK: u32 = 1 << 30;

pub struct ImeSession {
    pub(crate) transliterator: Transliterator,
    pub(crate) visible_refiner: Option<Arc<Transliterator>>,
    pub(crate) commit_refiner: Option<Transliterator>,
    pub(crate) history: HashMap<String, usize>,
    pub(crate) enabled: bool,
    pub(crate) focused: bool,
    pub(crate) composition_raw: String,
    pub(crate) candidates: Vec<String>,
    pub(crate) phrase_candidates: Vec<PhraseCandidate>,
    pub(crate) selected_phrase_index: usize,
    pub(crate) selected_index: usize,
    pub(crate) selection_touched: bool,
    pub(crate) segmented_session: Option<SegmentedSession>,
    pub(crate) segment_edit_state: Option<SegmentEditState>,
    pub(crate) visible_refined_segments: Option<Vec<String>>,
    pub(crate) cursor_location: CursorLocation,
    pub(crate) input_mode: InputMode,
    pub(crate) options: ImeSessionOptions,
}

pub struct ImeSessionBuilder {
    transliterator: Transliterator,
    visible_refiner: Option<Transliterator>,
    commit_refiner: Option<Transliterator>,
    history: HashMap<String, usize>,
    input_mode: InputMode,
    options: ImeSessionOptions,
}

impl ImeSessionBuilder {
    pub fn visible_refiner(mut self, visible_refiner: Transliterator) -> Self {
        self.visible_refiner = Some(visible_refiner);
        self
    }

    pub fn commit_refiner(mut self, commit_refiner: Transliterator) -> Self {
        self.commit_refiner = Some(commit_refiner);
        self
    }

    pub fn input_mode(mut self, input_mode: InputMode) -> Self {
        self.input_mode = input_mode;
        self
    }

    pub fn options(mut self, options: ImeSessionOptions) -> Self {
        self.options = options;
        self
    }

    pub fn build(self) -> ImeSession {
        let mut session = ImeSession::new_with_optional_refiners(
            self.transliterator,
            self.visible_refiner,
            self.commit_refiner,
            self.history,
        );
        session.input_mode = self.input_mode;
        session.options = self.options;
        session
    }
}

impl ImeSession {
    pub fn builder(transliterator: Transliterator, history: HashMap<String, usize>) -> ImeSessionBuilder {
        ImeSessionBuilder {
            transliterator,
            visible_refiner: None,
            commit_refiner: None,
            history,
            input_mode: InputMode::Roman,
            options: ImeSessionOptions::default(),
        }
    }

    pub fn new(transliterator: Transliterator, history: HashMap<String, usize>) -> Self {
        Self::new_with_optional_refiners(transliterator, None, None, history)
    }

    #[deprecated(note = "use ImeSession::builder(...).input_mode(...).build()")]
    pub fn new_with_input_mode(
        transliterator: Transliterator,
        history: HashMap<String, usize>,
        input_mode: InputMode,
    ) -> Self {
        let mut session = Self::new_with_optional_refiners(transliterator, None, None, history);
        session.input_mode = input_mode;
        session
    }

    #[deprecated(note = "use ImeSession::builder(...).input_mode(...).options(...).build()")]
    pub fn new_with_input_mode_and_options(
        transliterator: Transliterator,
        history: HashMap<String, usize>,
        input_mode: InputMode,
        options: ImeSessionOptions,
    ) -> Self {
        let mut session = Self::new_with_optional_refiners(transliterator, None, None, history);
        session.input_mode = input_mode;
        session.options = options;
        session
    }

    #[deprecated(note = "use ImeSession::builder(...).commit_refiner(...).build()")]
    pub fn new_with_commit_refiner(
        transliterator: Transliterator,
        commit_refiner: Transliterator,
        history: HashMap<String, usize>,
    ) -> Self {
        Self::new_with_optional_refiners(transliterator, None, Some(commit_refiner), history)
    }

    #[deprecated(note = "use ImeSession::builder(...).commit_refiner(...).input_mode(...).build()")]
    pub fn new_with_commit_refiner_and_input_mode(
        transliterator: Transliterator,
        commit_refiner: Transliterator,
        history: HashMap<String, usize>,
        input_mode: InputMode,
    ) -> Self {
        let mut session = Self::new_with_optional_refiners(transliterator, None, Some(commit_refiner), history);
        session.input_mode = input_mode;
        session
    }

    #[deprecated(note = "use ImeSession::builder(...).commit_refiner(...).input_mode(...).options(...).build()")]
    pub fn new_with_commit_refiner_input_mode_and_options(
        transliterator: Transliterator,
        commit_refiner: Transliterator,
        history: HashMap<String, usize>,
        input_mode: InputMode,
        options: ImeSessionOptions,
    ) -> Self {
        let mut session = Self::new_with_optional_refiners(transliterator, None, Some(commit_refiner), history);
        session.input_mode = input_mode;
        session.options = options;
        session
    }

    #[deprecated(note = "use ImeSession::builder(...).visible_refiner(...).commit_refiner(...).build()")]
    pub fn new_with_visible_and_commit_refiners(
        transliterator: Transliterator,
        visible_refiner: Transliterator,
        commit_refiner: Transliterator,
        history: HashMap<String, usize>,
    ) -> Self {
        Self::new_with_optional_refiners(transliterator, Some(visible_refiner), Some(commit_refiner), history)
    }

    #[deprecated(
        note = "use ImeSession::builder(...).visible_refiner(...).commit_refiner(...).input_mode(...).options(...).build()"
    )]
    pub fn new_with_visible_and_commit_refiners_input_mode_and_options(
        transliterator: Transliterator,
        visible_refiner: Transliterator,
        commit_refiner: Transliterator,
        history: HashMap<String, usize>,
        input_mode: InputMode,
        options: ImeSessionOptions,
    ) -> Self {
        let mut session =
            Self::new_with_optional_refiners(transliterator, Some(visible_refiner), Some(commit_refiner), history);
        session.input_mode = input_mode;
        session.options = options;
        session
    }

    fn new_with_optional_refiners(
        transliterator: Transliterator,
        visible_refiner: Option<Transliterator>,
        commit_refiner: Option<Transliterator>,
        history: HashMap<String, usize>,
    ) -> Self {
        Self {
            transliterator,
            visible_refiner: visible_refiner.map(Arc::new),
            commit_refiner,
            history,
            enabled: true,
            focused: false,
            composition_raw: String::new(),
            candidates: Vec::new(),
            phrase_candidates: Vec::new(),
            selected_phrase_index: 0,
            selected_index: 0,
            selection_touched: false,
            segmented_session: None,
            segment_edit_state: None,
            visible_refined_segments: None,
            cursor_location: CursorLocation::default(),
            input_mode: InputMode::Roman,
            options: ImeSessionOptions::default(),
        }
    }

    pub fn from_store<S: crate::adapter_contract::HistoryStore>(
        transliterator: Transliterator,
        store: &S,
    ) -> Result<Self, S::Error> {
        let history = store.load()?;
        Ok(Self::new(transliterator, history))
    }

    pub fn save_history<S: crate::adapter_contract::HistoryStore>(&self, store: &S) -> Result<(), S::Error> {
        store.save(&self.history)
    }

    pub fn composition_is_empty(&self) -> bool {
        self.composition_raw.is_empty()
    }

    pub fn composition_raw(&self) -> &str {
        &self.composition_raw
    }

    pub fn segmented_preview_active(&self) -> bool {
        self.segmented_session.is_some()
    }

    /// Whether a Model-mode visible refiner is attached. Adapters read this as the
    /// Standard/Smart state instead of tracking a separate flag.
    pub fn visible_refiner_active(&self) -> bool {
        self.visible_refiner.is_some()
    }

    #[deprecated(note = "install live, visible, and commit engines together with replace_engines_with_refiners")]
    pub fn set_commit_refiner(&mut self, commit_refiner: Transliterator) {
        self.commit_refiner = Some(commit_refiner);
    }

    #[deprecated(note = "install live, visible, and commit engines together with replace_engines_with_refiners")]
    pub fn set_visible_refiner(&mut self, visible_refiner: Transliterator) {
        self.visible_refiner = Some(Arc::new(visible_refiner));
    }

    pub fn replace_engines(
        &mut self,
        transliterator: Transliterator,
        commit_refiner: Option<Transliterator>,
        segmented_preview: SegmentedPreviewMode,
    ) {
        self.transliterator = transliterator;
        self.visible_refiner = None;
        self.commit_refiner = commit_refiner;
        self.options.segmented_preview = segmented_preview;
        if self.options.segmented_preview == SegmentedPreviewMode::Disabled {
            self.segmented_session = None;
        }
    }

    pub fn replace_engines_with_refiners(
        &mut self,
        transliterator: Transliterator,
        visible_refiner: Option<Transliterator>,
        commit_refiner: Option<Transliterator>,
        segmented_preview: SegmentedPreviewMode,
    ) {
        self.transliterator = transliterator;
        self.visible_refiner = visible_refiner.map(Arc::new);
        self.commit_refiner = commit_refiner;
        self.options.segmented_preview = segmented_preview;
        if self.options.segmented_preview == SegmentedPreviewMode::Disabled {
            self.segmented_session = None;
        }
    }

    #[deprecated(note = "install live, visible, and commit engines together with replace_engines_with_refiners")]
    pub fn replace_live_transliterator(
        &mut self,
        transliterator: Transliterator,
        segmented_preview: SegmentedPreviewMode,
    ) {
        self.transliterator = transliterator;
        self.options.segmented_preview = segmented_preview;
        if self.options.segmented_preview == SegmentedPreviewMode::Disabled {
            self.segmented_session = None;
        }
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
        self.phrase_candidates.clear();
        self.selected_phrase_index = 0;
        self.selected_index = 0;
        self.selection_touched = false;
        self.segmented_session = None;
        self.segment_edit_state = None;
        self.visible_refined_segments = None;
    }

    /// Clears all learned per-user history (backs the Settings "Clear Learned
    /// History" action). Callers persist the cleared state with `save_history`.
    pub fn reset_history(&mut self) {
        self.history.clear();
    }

    pub fn set_cursor_location(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.cursor_location = CursorLocation { x, y, width, height };
    }

    pub fn input_mode(&self) -> InputMode {
        self.input_mode
    }

    pub fn set_input_mode(&mut self, input_mode: InputMode) {
        if self.input_mode == input_mode {
            return;
        }
        self.input_mode = input_mode;
        self.reset();
    }

    pub fn toggle_input_mode(&mut self) {
        let next = match self.input_mode {
            InputMode::Roman => InputMode::Nida,
            InputMode::Nida | InputMode::CharPick => InputMode::Roman,
        };
        self.set_input_mode(next);
    }

    pub fn history(&self) -> &HashMap<String, usize> {
        &self.history
    }

    pub fn process_command(&mut self, command: SessionCommand) -> SessionResult {
        match command {
            SessionCommand::ProcessKeyEvent(event) => self.process_native_key_event(event),
            SessionCommand::SetInputMode(input_mode) => {
                self.set_input_mode(input_mode);
                SessionResult::default()
            }
            SessionCommand::ToggleInputMode => {
                self.toggle_input_mode();
                SessionResult {
                    consumed: true,
                    ..SessionResult::default()
                }
            }
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
            SessionCommand::SelectPhrase(index) => self.select_phrase(index),
        }
    }

    pub fn process_native_key_event(&mut self, event: NativeKeyEvent) -> SessionResult {
        self.process_key_event(event.keyval, event.keycode, event.state)
    }

    // Run the candidate decode for the current composition. Pairs with
    // `process_printable_deferred`: call this once typing pauses so the UI can
    // replace its preserved suggestions with current candidates and segments.
    pub fn recompute_now(&mut self) -> SessionResult {
        if self.composition_raw.is_empty() {
            return SessionResult::default();
        }
        self.recompute_composition_state();
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    pub fn process_key_event(&mut self, keyval: u32, keycode: u32, state: u32) -> SessionResult {
        if !self.enabled {
            return SessionResult::default();
        }
        if !self.focused {
            self.focused = true;
        }

        if is_modifier_only(state) || is_key_release(state) {
            return SessionResult::default();
        }

        if keyval == KEY_CAPS_LOCK {
            self.toggle_input_mode();
            return SessionResult {
                consumed: true,
                ..SessionResult::default()
            };
        }

        if self.input_mode == InputMode::Nida {
            return self.process_nida_key_event(keyval, keycode, state);
        }

        if self.input_mode == InputMode::CharPick {
            return self.process_char_pick_key_event(keyval);
        }

        match keyval {
            KEY_LEFT => self.handle_left(),
            KEY_TAB => self.handle_tab(),
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

    pub fn apply_refined_candidate(&mut self, raw_preedit: &str) -> Option<String> {
        if self.input_mode != InputMode::Roman
            || raw_preedit.is_empty()
            || raw_preedit != self.composition_raw
            || self.segmented_session.is_some()
            || self.selection_touched
        {
            return None;
        }

        let refined_segments = self.visible_refined_phrase_segments_for(raw_preedit)?;
        let refined = refined_segments.concat();
        if refined == raw_preedit {
            return None;
        }

        let refined_key = normalized_suggestion_key(&refined);
        self.candidates
            .retain(|candidate| normalized_suggestion_key(candidate) != refined_key);
        self.candidates.insert(0, refined.clone());
        self.selected_index = 0;
        self.visible_refined_segments = Some(refined_segments);
        Some(refined)
    }

    fn handle_printable(&mut self, ch: char) -> SessionResult {
        self.handle_printable_inner(ch, false)
    }

    // Append a roman char WITHOUT running the expensive candidate decode. The
    // caller must run `recompute_now()` once typing pauses to produce candidates.
    // Used by the Android deferred path so fast typing doesn't pay the per-key
    // decode (300–800 ms) it was queueing before. Single-keycap auto-commit
    // (digit/symbol) still decodes eagerly — it needs candidates to fire and is
    // cheap on a 1-char composition.
    pub fn process_printable_deferred(&mut self, ch: char) -> SessionResult {
        self.handle_printable_inner(ch, true)
    }

    fn handle_printable_inner(&mut self, ch: char, deferred: bool) -> SessionResult {
        let normalized = if ch.is_ascii_alphabetic() {
            ch.to_ascii_lowercase()
        } else {
            ch
        };
        if self.segment_edit_state.is_some() {
            return self.handle_segment_edit_printable(normalized);
        }
        self.composition_raw.push(normalized);
        // Skip the decode when deferring, unless this could be a single-keycap
        // auto-commit (needs candidates now, and a 1-char decode is cheap).
        let eager = !deferred
            || (self.composition_raw.chars().count() == 1 && is_single_keycap_char(normalized));
        if eager {
            self.recompute_composition_state();
        }
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

    fn handle_up(&mut self) -> SessionResult {
        self.cycle_candidates(-1)
    }

    fn handle_down(&mut self) -> SessionResult {
        self.cycle_candidates(1)
    }

    fn handle_space(&mut self) -> SessionResult {
        if self.segment_edit_state.is_some() {
            return self.commit_selected_or_raw();
        }
        self.cycle_candidates(1)
    }

    fn handle_backspace(&mut self) -> SessionResult {
        if self.segment_edit_state.is_some() {
            return self.handle_segment_edit_backspace();
        }

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
        if let Some(edit_state) = self.segment_edit_state.take() {
            if let Some(session) = &mut self.segmented_session {
                if edit_state.index < session.segments.len() {
                    session.segments[edit_state.index] = edit_state.original_segment;
                    session.focused = edit_state.index;
                    self.composition_raw = recompute_segment_ranges_and_raw(session);
                    session.raw_input = self.composition_raw.clone();
                }
            }
            return SessionResult {
                consumed: true,
                ..SessionResult::default()
            };
        }

        if self.composition_raw.is_empty() {
            return SessionResult::default();
        }
        self.reset();
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    fn handle_digit(&mut self, slot: usize, keyval: u32) -> SessionResult {
        if self.composition_raw.is_empty() {
            if let Some(ch) = keyval_to_ascii_char(keyval) {
                return self.handle_printable(ch);
            }
            return SessionResult::default();
        }

        // A digit only selects a row the current page actually paints. On a
        // 9-row page the '0' key (slot 9) is not a selector and falls through to
        // being typed literally, so adapters that keep `page_size` at 9 behave
        // exactly as they did before pagination existed.
        let page_size = self.options.page_size.max(1);
        if slot >= page_size {
            if let Some(ch) = keyval_to_ascii_char(keyval) {
                return self.handle_printable(ch);
            }
            return SessionResult::default();
        }

        if let Some(edit_state) = &self.segment_edit_state {
            self.select_segment_candidate_without_reflow(edit_state.index, slot);
            self.selection_touched = true;
            return SessionResult {
                consumed: true,
                ..SessionResult::default()
            };
        }

        if let Some(session) = self.segmented_session.clone() {
            let Some(segment) = session.segments.get(session.focused) else {
                return SessionResult::default();
            };
            if slot < segment.candidates.len() {
                self.select_focused_segment_candidate(slot);
                self.selection_touched = true;
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

        // Flat Candidate List: the in-page slot maps to an absolute index using
        // the page the selection currently sits on, so digits pick the row the
        // user sees on the visible page rather than always page one.
        let absolute = (self.selected_index / page_size) * page_size + slot;
        if absolute < self.candidates.len() {
            self.selected_index = absolute;
            self.selection_touched = true;
        }
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
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
}

pub(crate) fn exact_matches_first(
    transliterator: &Transliterator,
    input: &str,
    candidates: Vec<String>,
) -> Vec<String> {
    let exact_keys = transliterator
        .exact_match_targets(input)
        .into_iter()
        .map(|item| normalized_suggestion_key(&item))
        .collect::<HashSet<_>>();
    if exact_keys.is_empty() {
        return candidates;
    }

    let mut exact = Vec::new();
    let mut fallback = Vec::new();
    for candidate in candidates {
        if exact_keys.contains(&normalized_suggestion_key(&candidate)) {
            exact.push(candidate);
        } else {
            fallback.push(candidate);
        }
    }
    exact.extend(fallback);
    exact
}

fn is_modifier_only(state: u32) -> bool {
    state & (STATE_CONTROL_MASK | STATE_MOD1_MASK | STATE_SUPER_MASK | STATE_HYPER_MASK | STATE_META_MASK) != 0
}

fn is_key_release(state: u32) -> bool {
    state & STATE_RELEASE_MASK != 0
}

fn keyval_to_digit_index(keyval: u32) -> Option<usize> {
    let ch = char::from_u32(keyval)?;
    if !ch.is_ascii_digit() {
        return None;
    }
    // In-page slot: '1'..='9' select rows 0..=8, '0' selects the tenth row.
    // `handle_digit` gates the slot against the active page size and maps it to
    // an absolute candidate index.
    Some(if ch == '0' { 9 } else { (ch as u8 - b'1') as usize })
}

fn keyval_to_ascii_char(keyval: u32) -> Option<char> {
    let ch = char::from_u32(keyval)?;
    if ch.is_ascii_graphic() {
        Some(ch)
    } else {
        None
    }
}

pub(crate) fn offset_index(current: usize, len: usize, delta: isize) -> usize {
    debug_assert!(len > 0);
    (current as isize + delta).rem_euclid(len as isize) as usize
}

pub(crate) fn recompute_segment_ranges_and_raw(session: &mut SegmentedSession) -> String {
    let mut raw = String::new();
    let mut cursor = 0;
    for segment in &mut session.segments {
        segment.start = cursor;
        cursor += segment.input.chars().count();
        segment.end = cursor;
        raw.push_str(&segment.input);
    }
    raw
}

fn is_single_keycap_char(ch: char) -> bool {
    matches!(
        ch,
        '0'..='9' | '!' | '@' | '"' | '#' | '$' | '%' | '^' | '&' | '*' | '\'' | '(' | ')' | '~' | '='
    )
}

#[cfg(test)]
mod tests {
    use super::ImeSession;

    use crate::adapter_contract::{InputMode, NativeKeyEvent, SessionCommand};
    use crate::test_support::{paged_session, session, type_ascii};

    #[test]
    fn deferred_printable_appends_without_decoding_then_recompute_matches_eager() {
        // Eager reference: type the whole word the normal way.
        let mut eager = session();
        type_ascii(&mut eager, "khnhom");
        let eager_candidates = eager.snapshot().candidates;
        assert!(!eager_candidates.is_empty(), "eager path must produce candidates");

        // Deferred: same chars, no per-key decode.
        let mut deferred = session();
        for ch in "khnhom".chars() {
            deferred.process_printable_deferred(ch);
        }
        assert_eq!(deferred.snapshot().raw_preedit, "khnhom", "roman keeps up");
        assert!(
            deferred.snapshot().candidates.is_empty(),
            "no decode ran during deferred typing"
        );

        // On pause, one recompute produces the same candidates as the eager path.
        deferred.recompute_now();
        assert_eq!(deferred.snapshot().candidates, eager_candidates);
    }

    #[test]
    fn deferred_single_keycap_still_auto_commits() {
        // A digit is a single-keycap auto-commit char; deferred must still decode
        // it (1-char, cheap) so `1` -> `១` commits instantly.
        let mut session = session();
        let result = session.process_printable_deferred('1');
        assert_eq!(result.commit_text.as_deref(), Some("១"));
        assert!(session.snapshot().preedit.is_empty());
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
    fn session_defaults_to_roman_input_mode() {
        let session = session();

        assert_eq!(session.input_mode(), InputMode::Roman);
        assert_eq!(session.snapshot().input_mode, InputMode::Roman);
    }

    #[test]
    fn builder_configures_input_mode_options_and_refiners() {
        let transliterator =
            khmerime_core::Transliterator::from_default_data_with_config(khmerime_core::DecoderConfig::legacy())
                .expect("default data must load");
        let visible_refiner =
            khmerime_core::Transliterator::from_default_data_with_config(khmerime_core::DecoderConfig::legacy())
                .expect("default data must load");
        let commit_refiner =
            khmerime_core::Transliterator::from_default_data_with_config(khmerime_core::DecoderConfig::legacy())
                .expect("default data must load");

        let session = ImeSession::builder(transliterator, std::collections::HashMap::new())
            .input_mode(InputMode::Nida)
            .options(crate::adapter_contract::ImeSessionOptions {
                segmented_preview: crate::adapter_contract::SegmentedPreviewMode::Disabled,
                ..Default::default()
            })
            .visible_refiner(visible_refiner)
            .commit_refiner(commit_refiner)
            .build();

        assert_eq!(session.input_mode(), InputMode::Nida);
        assert_eq!(
            session.options.segmented_preview,
            crate::adapter_contract::SegmentedPreviewMode::Disabled
        );
        assert!(session.visible_refiner.is_some());
        assert!(session.commit_refiner.is_some());
    }

    #[test]
    fn set_input_mode_clears_active_composition() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        assert_eq!(session.snapshot().raw_preedit, "jea");

        let update = session.process_command(SessionCommand::SetInputMode(InputMode::Nida));

        assert_eq!(update, Default::default());
        let snapshot = session.snapshot();
        assert_eq!(snapshot.input_mode, InputMode::Nida);
        assert!(snapshot.raw_preedit.is_empty());
        assert!(snapshot.candidates.is_empty());
        assert!(!snapshot.segmented_active);
    }

    #[test]
    fn caps_lock_toggles_input_mode_and_consumes_key() {
        let mut session = session();
        type_ascii(&mut session, "jea");

        let update = session.process_key_event(0xFFE5, 0, 0);

        assert!(update.consumed);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.input_mode, InputMode::Nida);
        assert!(snapshot.raw_preedit.is_empty());
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
    fn raw_fallback_is_flagged_as_last_candidate() {
        let mut session = paged_session(10);
        type_ascii(&mut session, "sa");
        let snapshot = session.snapshot();
        let flagged = snapshot
            .candidate_display
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.is_raw_fallback)
            .collect::<Vec<_>>();
        assert_eq!(flagged.len(), 1, "exactly one candidate is the raw fallback");
        let (index, entry) = flagged[0];
        assert_eq!(entry.output, "sa", "the raw fallback carries the literal roman input");
        assert_eq!(index, snapshot.candidates.len() - 1, "the raw fallback is pinned last");
    }

    #[test]
    fn digit_zero_selects_tenth_row_when_page_size_is_ten() {
        let mut session = paged_session(10);
        type_ascii(&mut session, "sa");
        assert!(session.snapshot().candidates.len() >= 10, "need a full page");
        let update = session.process_key_event('0' as u32, 0, 0);
        assert!(update.consumed);
        assert_eq!(session.snapshot().selected_index, Some(9), "'0' selects the tenth row");
    }

    #[test]
    fn digit_zero_types_literal_when_page_size_is_nine() {
        let mut session = paged_session(9);
        type_ascii(&mut session, "sa");
        let update = session.process_key_event('0' as u32, 0, 0);
        assert!(update.consumed);
        // Slot 9 is outside a 9-row page, so '0' falls through to being typed.
        assert_eq!(session.snapshot().raw_preedit, "sa0");
        assert_eq!(session.snapshot().selected_index, Some(0), "no selection changed");
    }

    #[test]
    fn digit_selection_is_page_relative_on_the_second_page() {
        let mut session = paged_session(10);
        type_ascii(&mut session, "sa");
        assert!(session.snapshot().candidates.len() >= 11, "need a second page");
        // Cycle the selection onto the second page (index 10).
        for _ in 0..10 {
            session.process_key_event(0x20, 0, 0);
        }
        assert_eq!(session.snapshot().selected_index, Some(10));
        // On page two, digit '1' selects the first visible row = absolute index 10.
        session.process_key_event('1' as u32, 0, 0);
        assert_eq!(session.snapshot().selected_index, Some(10));
        // '2' selects the second visible row = absolute index 11.
        session.process_key_event('2' as u32, 0, 0);
        assert_eq!(session.snapshot().selected_index, Some(11));
    }

    #[test]
    fn reset_history_clears_learned_history() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        session.process_key_event(0xFF0D, 0, 0); // commit learns "ជា" into history
        assert!(!session.history().is_empty(), "committing a word must learn history");

        session.reset_history();

        assert!(
            session.history().is_empty(),
            "reset_history must clear all learned history",
        );
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

    #[test]
    fn char_pick_mode_typing_k_returns_relation_candidates() {
        let mut session = session();
        session.process_command(SessionCommand::SetInputMode(InputMode::CharPick));

        let result = session.process_key_event('k' as u32, 0, 0);

        assert!(result.consumed);
        assert!(result.commit_text.is_none());
        let snapshot = session.snapshot();
        assert!(snapshot.preedit.is_empty(), "preedit must be empty in CharPick mode");
        assert!(
            snapshot.candidates.iter().any(|c| c == "ក"),
            "candidates must include ក for 'k', got: {:?}",
            snapshot.candidates
        );
        assert!(
            !snapshot.candidates.iter().any(|c| c == "ម"),
            "candidates must not include ម for 'k'"
        );
    }

    #[test]
    fn char_pick_mode_does_not_affect_roman_mode() {
        let mut session = session();
        // Roman mode: 'k' builds a Composition with roman preedit, not a char-pick lookup
        let result = session.process_key_event('k' as u32, 0, 0);
        assert!(result.consumed);
        // The roman path is confirmed by raw_preedit containing the typed letter
        assert_eq!(session.snapshot().raw_preedit, "k");
    }
}
