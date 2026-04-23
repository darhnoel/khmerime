use std::collections::HashMap;

use crate::engine;
use dioxus::prelude::*;
use roman_lookup::{
    build_segmented_session as build_shared_segmented_session, move_session_focus,
    reflow_segmented_session_from_selection as reflow_shared_segmented_session, DecoderMode, SegmentedSession,
    ShadowObservation,
};

use super::EditorSignals;

fn sync_refine_state(mut state: EditorSignals, session: SegmentedSession, selected: usize) {
    state.segmented_refine_mode.set(true);
    state.suggestions.set(session.focused_candidates());
    state.recommended_indices.set(Vec::new());
    state.roman_variant_hints.set(HashMap::new());
    state.selected.set(selected);
    state.selection_started.set(true);
    state.segmented_session.set(Some(session));
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
