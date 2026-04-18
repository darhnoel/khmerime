use std::collections::HashMap;

use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ManualComposeCandidate, ManualComposeKind, ShadowObservation};

use crate::{CompositionMark, SuggestionPopup};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SegmentedChoice {
    pub input: String,
    pub start: usize,
    pub end: usize,
    pub candidates: Vec<String>,
    pub selected: usize,
}

impl SegmentedChoice {
    pub(crate) fn selected_text(&self) -> String {
        self.candidates
            .get(self.selected)
            .cloned()
            .or_else(|| self.candidates.first().cloned())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SegmentedSession {
    pub raw_input: String,
    pub segments: Vec<SegmentedChoice>,
    pub focused: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputMode {
    NormalWordSuggestion,
    ManualCharacterTyping,
}

impl InputMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            InputMode::NormalWordSuggestion => "Word",
            InputMode::ManualCharacterTyping => "Manual",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManualTypingState {
    pub raw_roman: String,
    pub consumed: usize,
    pub composed_text: String,
    pub expected_kind: ManualComposeKind,
    pub kind_filter: ManualComposeKind,
    pub last_selected_base_consonant: Option<String>,
    pub context_fallback_key: Option<String>,
    pub context_fallback_insert_text: Option<String>,
    pub active_span: Option<std::ops::Range<usize>>,
    pub candidates: Vec<ManualComposeCandidate>,
    pub checkpoints: Vec<ManualTypingCheckpoint>,
}

impl ManualTypingState {
    pub(crate) fn new(raw_roman: String) -> Self {
        let mut state = Self {
            raw_roman,
            consumed: 0,
            composed_text: String::new(),
            expected_kind: ManualComposeKind::BaseConsonant,
            kind_filter: ManualComposeKind::BaseConsonant,
            last_selected_base_consonant: None,
            context_fallback_key: None,
            context_fallback_insert_text: None,
            active_span: None,
            candidates: Vec::new(),
            checkpoints: Vec::new(),
        };
        super::manual_flow::refresh_manual_state_candidates(&mut state);
        state
    }

    pub(crate) fn remaining_roman(&self) -> String {
        slice_chars(&self.raw_roman, self.consumed..char_len(&self.raw_roman))
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.consumed >= char_len(&self.raw_roman) && !self.composed_text.is_empty()
    }

    pub(crate) fn checkpoint(&self) -> ManualTypingCheckpoint {
        ManualTypingCheckpoint {
            consumed: self.consumed,
            composed_text: self.composed_text.clone(),
            expected_kind: self.expected_kind,
            kind_filter: self.kind_filter,
            last_selected_base_consonant: self.last_selected_base_consonant.clone(),
        }
    }

    pub(crate) fn restore_checkpoint(&mut self, checkpoint: ManualTypingCheckpoint) {
        self.consumed = checkpoint.consumed;
        self.composed_text = checkpoint.composed_text;
        self.expected_kind = checkpoint.expected_kind;
        self.kind_filter = checkpoint.kind_filter;
        self.last_selected_base_consonant = checkpoint.last_selected_base_consonant;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManualSaveRequest {
    pub roman: String,
    pub khmer: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManualTypingCheckpoint {
    pub(crate) consumed: usize,
    pub(crate) composed_text: String,
    pub(crate) expected_kind: ManualComposeKind,
    pub(crate) kind_filter: ManualComposeKind,
    pub(crate) last_selected_base_consonant: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct EditorSignals {
    pub text: Signal<String>,
    pub roman_enabled: Signal<bool>,
    pub input_mode: Signal<InputMode>,
    pub decoder_mode: Signal<DecoderMode>,
    pub engine_ready: Signal<bool>,
    pub engine_progress: Signal<u8>,
    pub suggestions: Signal<Vec<String>>,
    pub popup: Signal<Option<SuggestionPopup>>,
    pub composition: Signal<Option<CompositionMark>>,
    pub shadow_debug: Signal<Option<ShadowObservation>>,
    pub segmented_session: Signal<Option<SegmentedSession>>,
    pub segmented_refine_mode: Signal<bool>,
    pub active_token: Signal<String>,
    pub recommended_indices: Signal<Vec<usize>>,
    pub roman_variant_hints: Signal<HashMap<usize, Vec<String>>>,
    pub number_pick_mode: Signal<bool>,
    pub selection_started: Signal<bool>,
    pub selected: Signal<usize>,
    pub pending_caret: Signal<Option<usize>>,
    pub history: Signal<HashMap<String, usize>>,
    pub manual_typing_state: Signal<Option<ManualTypingState>>,
    pub manual_save_request: Signal<Option<ManualSaveRequest>>,
    pub user_dictionary: Signal<HashMap<String, Vec<String>>>,
}

impl EditorSignals {
    pub(crate) fn text(self) -> String {
        (self.text)()
    }

    pub(crate) fn roman_enabled(self) -> bool {
        (self.roman_enabled)()
    }

    pub(crate) fn input_mode(self) -> InputMode {
        (self.input_mode)()
    }

    pub(crate) fn decoder_mode(self) -> DecoderMode {
        (self.decoder_mode)()
    }

    pub(crate) fn engine_ready(self) -> bool {
        (self.engine_ready)()
    }

    pub(crate) fn suggestions(self) -> Vec<String> {
        (self.suggestions)()
    }

    pub(crate) fn popup(self) -> Option<SuggestionPopup> {
        (self.popup)()
    }

    pub(crate) fn composition(self) -> Option<CompositionMark> {
        (self.composition)()
    }

    pub(crate) fn shadow_debug(self) -> Option<ShadowObservation> {
        (self.shadow_debug)()
    }

    pub(crate) fn segmented_session(self) -> Option<SegmentedSession> {
        (self.segmented_session)()
    }

    pub(crate) fn segmented_refine_mode(self) -> bool {
        (self.segmented_refine_mode)()
    }

    pub(crate) fn active_token(self) -> String {
        (self.active_token)()
    }

    pub(crate) fn number_pick_mode(self) -> bool {
        (self.number_pick_mode)()
    }

    pub(crate) fn recommended_indices(self) -> Vec<usize> {
        (self.recommended_indices)()
    }

    pub(crate) fn roman_variant_hints(self) -> HashMap<usize, Vec<String>> {
        (self.roman_variant_hints)()
    }

    pub(crate) fn selection_started(self) -> bool {
        (self.selection_started)()
    }

    pub(crate) fn selected(self) -> usize {
        (self.selected)()
    }

    pub(crate) fn history(self) -> HashMap<String, usize> {
        (self.history)()
    }

    pub(crate) fn manual_typing_state(self) -> Option<ManualTypingState> {
        (self.manual_typing_state)()
    }

    pub(crate) fn manual_save_request(self) -> Option<ManualSaveRequest> {
        (self.manual_save_request)()
    }

    pub(crate) fn user_dictionary(self) -> HashMap<String, Vec<String>> {
        (self.user_dictionary)()
    }

    pub(crate) fn clear_candidate_state(mut self) {
        self.suggestions.set(Vec::new());
        self.popup.set(None);
        self.composition.set(None);
        self.shadow_debug.set(None);
        self.segmented_session.set(None);
        self.segmented_refine_mode.set(false);
        self.active_token.set(String::new());
        self.recommended_indices.set(Vec::new());
        self.roman_variant_hints.set(HashMap::new());
        self.selection_started.set(false);
        self.selected.set(0);
        self.manual_typing_state.set(None);
    }

    pub(crate) fn clear_candidate_state_and_picker(mut self) {
        self.clear_candidate_state();
        self.number_pick_mode.set(false);
    }
}

impl SegmentedSession {
    pub(crate) fn focused_candidates(&self) -> Vec<String> {
        self.segments
            .get(self.focused)
            .map(|segment| segment.candidates.clone())
            .unwrap_or_default()
    }

    pub(crate) fn current_candidate_len(&self) -> usize {
        self.segments
            .get(self.focused)
            .map(|segment| segment.candidates.len())
            .unwrap_or(0)
    }

    pub(crate) fn focused_selected(&self) -> usize {
        self.segments
            .get(self.focused)
            .map(|segment| segment.selected)
            .unwrap_or(0)
    }

    pub(crate) fn composed_text(&self) -> String {
        self.segments
            .iter()
            .map(SegmentedChoice::selected_text)
            .collect::<String>()
    }
}

pub(crate) fn slice_chars(input: &str, range: std::ops::Range<usize>) -> String {
    input
        .chars()
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
        .collect()
}

pub(crate) fn char_len(input: &str) -> usize {
    input.chars().count()
}
