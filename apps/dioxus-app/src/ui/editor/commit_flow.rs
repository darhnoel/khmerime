use dioxus::prelude::*;
use roman_lookup::Transliterator;

use crate::ui::platform::current_editor_caret;
use crate::ui::storage::{save_editor_text, save_history};

use super::candidate_pipeline::update_candidates;
use super::manual_flow::commit_manual_selection;
use super::segmented_flow::select_segment_candidate;
use super::{EditorSignals, InputMode};

pub(crate) fn switch_input_mode(next_mode: InputMode, mut state: EditorSignals) {
    if state.input_mode() == next_mode {
        return;
    }
    state.input_mode.set(next_mode);
    state.manual_save_request.set(None);
    state.clear_candidate_state_and_picker();
    if state.roman_enabled() {
        spawn(update_candidates(state.text(), state));
    }
}

pub(crate) async fn commit_active_selection(typed_space: bool, state: EditorSignals) {
    if state.input_mode() == InputMode::ManualCharacterTyping {
        commit_manual_selection(typed_space, state).await;
        return;
    }
    if state.segmented_refine_mode() && state.segmented_session().is_some() {
        commit_segmented_selection(typed_space, state).await;
        return;
    }
    commit_selection(typed_space, state).await;
}

pub(crate) async fn click_candidate(candidate_index: usize, mut state: EditorSignals) {
    if state.input_mode() == InputMode::ManualCharacterTyping {
        if candidate_index < state.suggestions().len() {
            state.selected.set(candidate_index);
            state.selection_started.set(true);
            commit_manual_selection(false, state).await;
        }
        return;
    }
    if state.segmented_refine_mode() && state.segmented_session().is_some() {
        if select_segment_candidate(candidate_index, state) {
            state.number_pick_mode.set(false);
        }
        return;
    }
    if candidate_index < state.suggestions().len() {
        state.selected.set(candidate_index);
        state.selection_started.set(true);
        commit_selection(false, state).await;
    }
}

pub(crate) async fn commit_segmented_selection(typed_space: bool, mut state: EditorSignals) {
    let Some(session) = state.segmented_session() else {
        return;
    };
    let choice = session.composed_text();
    if choice.is_empty() {
        return;
    }
    let current_text = state.text();
    let caret = current_editor_caret()
        .await
        .unwrap_or_else(|| current_text.chars().count());
    let applied = Transliterator::apply_suggestion(&current_text, caret, &choice, typed_space);

    let mut next_history = state.history();
    Transliterator::learn(&mut next_history, &choice);
    save_history(&next_history);
    state.history.set(next_history);

    save_editor_text(&applied.text);
    state.text.set(applied.text);
    state.clear_candidate_state();
    state.pending_caret.set(Some(applied.caret));
}

pub(crate) async fn commit_selection(typed_space: bool, mut state: EditorSignals) {
    let items = state.suggestions();
    if items.is_empty() {
        return;
    }
    let Some(choice) = items.get(state.selected()).cloned() else {
        return;
    };
    let current_text = state.text();
    let caret = current_editor_caret()
        .await
        .unwrap_or_else(|| current_text.chars().count());
    let applied = Transliterator::apply_suggestion(&current_text, caret, &choice, typed_space);

    let mut next_history = state.history();
    Transliterator::learn(&mut next_history, &choice);
    save_history(&next_history);
    state.history.set(next_history);

    save_editor_text(&applied.text);
    state.text.set(applied.text);
    state.clear_candidate_state();
    state.pending_caret.set(Some(applied.caret));
}
