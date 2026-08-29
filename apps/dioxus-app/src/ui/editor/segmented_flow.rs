use std::collections::HashMap;

use crate::engine;
use dioxus::prelude::*;
use roman_lookup::{
    build_segmented_session as build_shared_segmented_session, move_session_focus,
    reflow_segmented_session_from_selection as reflow_shared_segmented_session, DecodeCandidate, DecoderMode,
    SegmentedChoice, SegmentedSession, ShadowObservation,
};

use super::{CandidateLevel, EditorSignals};

fn sync_refine_state(mut state: EditorSignals, session: SegmentedSession, selected: usize) {
    state.segmented_refine_mode.set(true);
    state.suggestions.set(session.focused_candidates());
    state.recommended_indices.set(Vec::new());
    state.roman_variant_hints.set(HashMap::new());
    state.selected.set(selected);
    state.selection_started.set(true);
    state.segmented_session.set(Some(session));
    state.candidate_level.set(CandidateLevel::Segment);
}

fn phrase_session(
    candidate: &DecodeCandidate,
    raw_input: &str,
    history: &HashMap<String, usize>,
) -> Option<SegmentedSession> {
    if candidate.segments.len() < 2 {
        return None;
    }
    let legacy = engine(DecoderMode::Legacy);
    let mut start = 0usize;
    let segments = candidate
        .segments
        .iter()
        .map(|segment| {
            let len = segment.input.chars().count();
            let output = roman_lookup::connect_khmer_display(&segment.output);
            let mut candidates = roman_lookup::normalize_visible_suggestions(legacy.suggest(&segment.input, history));
            if let Some(index) = candidates.iter().position(|item| item == &output) {
                let preferred = candidates.remove(index);
                candidates.insert(0, preferred);
            } else {
                candidates.insert(0, output);
            }
            candidates.truncate(10);
            let choice = SegmentedChoice {
                input: segment.input.clone(),
                start,
                end: start + len,
                candidates,
                selected: 0,
            };
            start += len;
            choice
        })
        .collect();
    Some(SegmentedSession {
        raw_input: raw_input.to_owned(),
        segments,
        focused: 0,
    })
}

pub(crate) fn enter_segment_edit(index: usize, mut state: EditorSignals) -> bool {
    let phrases = state.phrase_candidates();
    let Some(candidate) = phrases.get(index) else {
        return false;
    };
    let Some(session) = phrase_session(candidate, &state.active_token(), &state.history()) else {
        return false;
    };
    state.active_phrase_index.set(index);
    sync_refine_state(state, session, 0);
    state.number_pick_mode.set(false);
    true
}

pub(crate) fn exit_segment_edit(mut state: EditorSignals) -> bool {
    if state.candidate_level() != CandidateLevel::Segment {
        return false;
    }
    let phrases = state.phrase_candidates();
    if phrases.is_empty() {
        return false;
    }
    let selected = state.active_phrase_index().min(phrases.len().saturating_sub(1));
    state
        .suggestions
        .set(phrases.iter().map(|candidate| candidate.text.clone()).collect());
    state.segmented_session.set(None);
    state.segmented_refine_mode.set(false);
    state.candidate_level.set(CandidateLevel::Phrase);
    state.selected.set(selected);
    state.selection_started.set(true);
    state.number_pick_mode.set(false);
    true
}

pub(crate) fn move_segment_focus(delta: isize, state: EditorSignals) -> bool {
    let Some(mut session) = state.segmented_session() else {
        return false;
    };

    let moved = move_session_focus(&mut session, delta);
    if !moved && state.segmented_refine_mode() {
        return false;
    }

    let selected = session.focused_selected();
    sync_refine_state(state, session, selected);
    true
}

pub(crate) fn select_segment_candidate(candidate_index: usize, state: EditorSignals) -> bool {
    let Some(mut session) = state.segmented_session() else {
        return false;
    };
    let focused = session.focused;
    let Some(segment) = session.segments.get(focused) else {
        return false;
    };
    if candidate_index >= segment.candidates.len() {
        return false;
    }

    session.segments[focused].selected = candidate_index;
    let next_session =
        reflow_segmented_session_from_selection(&session, &state.history()).unwrap_or_else(|| session.clone());
    let selected = next_session.focused_selected();
    sync_refine_state(state, next_session, selected);
    true
}

pub(super) fn build_segmented_session(
    observation: &ShadowObservation,
    raw_input: &str,
    history: &HashMap<String, usize>,
) -> Option<SegmentedSession> {
    let legacy = engine(DecoderMode::Legacy);
    build_shared_segmented_session(observation, raw_input, history, &|input, history| {
        legacy.suggest(input, history)
    })
}

pub(super) fn reflow_segmented_session_from_selection(
    session: &SegmentedSession,
    history: &HashMap<String, usize>,
) -> Option<SegmentedSession> {
    let legacy = engine(DecoderMode::Legacy);
    let shadow = engine(DecoderMode::Shadow);
    reflow_shared_segmented_session(
        session,
        history,
        &|input, history| legacy.suggest(input, history),
        &|input, target| legacy.best_prefix_consumption(input, target),
        &|input, history| shadow.shadow_observation(input, history),
    )
}
