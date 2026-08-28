use std::collections::HashMap;

use dioxus::prelude::*;
use roman_lookup::{DecodeCandidate, DecoderMode, SegmentedSession, ShadowObservation};

use crate::ui::spellcheck::SpellReview;
use crate::{CompositionMark, EngineReadiness, SuggestionPopup};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CandidateMode {
    None,
    Transliteration,
    NextWord,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CandidateLevel {
    #[default]
    Flat,
    Phrase,
    Segment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManualSaveRequest {
    pub roman: String,
    pub khmer: String,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct EditorSignals {
    pub text: Signal<String>,
    pub roman_enabled: Signal<bool>,
    pub decoder_mode: Signal<DecoderMode>,
    pub engine_readiness: Signal<EngineReadiness>,
    pub engine_ready: Signal<bool>,
    pub engine_progress: Signal<u8>,
    pub suggestions: Signal<Vec<String>>,
    pub popup: Signal<Option<SuggestionPopup>>,
    pub composition: Signal<Option<CompositionMark>>,
    pub shadow_debug: Signal<Option<ShadowObservation>>,
    pub segmented_session: Signal<Option<SegmentedSession>>,
    pub segmented_refine_mode: Signal<bool>,
    pub phrase_candidates: Signal<Vec<DecodeCandidate>>,
    pub candidate_level: Signal<CandidateLevel>,
    pub active_phrase_index: Signal<usize>,
    pub suggestion_loading: Signal<bool>,
    pub suggestion_request_id: Signal<u64>,
    pub candidate_mode: Signal<CandidateMode>,
    pub active_token: Signal<String>,
    pub recommended_indices: Signal<Vec<usize>>,
    pub roman_variant_hints: Signal<HashMap<usize, Vec<String>>>,
    pub number_pick_mode: Signal<bool>,
    pub selection_started: Signal<bool>,
    pub selected: Signal<usize>,
    pub pending_caret: Signal<Option<usize>>,
    pub pending_caret_no_focus: Signal<Option<usize>>,
    pub history: Signal<HashMap<String, usize>>,
    pub user_dictionary: Signal<HashMap<String, Vec<String>>>,
    pub spell_review: Signal<SpellReview>,
}

impl EditorSignals {
    pub(crate) fn text(self) -> String {
        (self.text)()
    }

    pub(crate) fn roman_enabled(self) -> bool {
        (self.roman_enabled)()
    }

    pub(crate) fn decoder_mode(self) -> DecoderMode {
        (self.decoder_mode)()
    }

    pub(crate) fn engine_readiness(self) -> EngineReadiness {
        (self.engine_readiness)()
    }

    pub(crate) fn engine_full_ready(self) -> bool {
        self.engine_readiness() == EngineReadiness::FullReady
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

    pub(crate) fn phrase_candidates(self) -> Vec<DecodeCandidate> {
        (self.phrase_candidates)()
    }

    pub(crate) fn candidate_level(self) -> CandidateLevel {
        (self.candidate_level)()
    }

    pub(crate) fn active_phrase_index(self) -> usize {
        (self.active_phrase_index)()
    }

    pub(crate) fn suggestion_loading(self) -> bool {
        (self.suggestion_loading)()
    }

    pub(crate) fn suggestion_request_id(self) -> u64 {
        (self.suggestion_request_id)()
    }

    pub(crate) fn candidate_mode(self) -> CandidateMode {
        (self.candidate_mode)()
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

    pub(crate) fn user_dictionary(self) -> HashMap<String, Vec<String>> {
        (self.user_dictionary)()
    }

    pub(crate) fn spell_review(self) -> SpellReview {
        (self.spell_review)()
    }

    pub(crate) fn clear_spell_review(mut self) {
        self.spell_review.set(SpellReview::default());
    }

    pub(crate) fn clear_candidate_state(mut self) {
        self.suggestions.set(Vec::new());
        self.popup.set(None);
        self.composition.set(None);
        self.shadow_debug.set(None);
        self.segmented_session.set(None);
        self.segmented_refine_mode.set(false);
        self.phrase_candidates.set(Vec::new());
        self.candidate_level.set(CandidateLevel::Flat);
        self.active_phrase_index.set(0);
        self.suggestion_loading.set(false);
        self.candidate_mode.set(CandidateMode::None);
        self.active_token.set(String::new());
        self.recommended_indices.set(Vec::new());
        self.roman_variant_hints.set(HashMap::new());
        self.selection_started.set(false);
        self.selected.set(0);
    }

    pub(crate) fn clear_candidate_state_and_picker(mut self) {
        self.clear_candidate_state();
        self.number_pick_mode.set(false);
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
