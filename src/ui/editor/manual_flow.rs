use std::collections::HashMap;

use dioxus::prelude::*;
use roman_lookup::{suggest_manual_character_candidates, ManualComposeCandidate, ManualComposeKind, Transliterator};

use crate::ui::platform::current_editor_caret;
use crate::ui::storage::{save_editor_text, save_history, save_user_dictionary};

use super::candidate_pipeline::{normalize_user_dictionary_key, update_candidates};
use super::{char_len, EditorSignals, ManualSaveRequest, ManualTypingState};

pub(super) fn manual_filtered_candidates(state: &ManualTypingState) -> Vec<ManualComposeCandidate> {
    state
        .candidates
        .iter()
        .filter(|candidate| candidate.kind == state.kind_filter)
        .cloned()
        .collect::<Vec<_>>()
}

pub(super) fn apply_manual_candidates(mut state: EditorSignals, manual: ManualTypingState, selection_started: bool) {
    let (items, hints) = manual_state_visible_candidates(&manual);
    state.manual_typing_state.set(Some(manual));
    state.segmented_refine_mode.set(false);
    state.segmented_session.set(None);
    state.recommended_indices.set(Vec::new());
    state.roman_variant_hints.set(hints);
    state.suggestions.set(items);
    state.selected.set(0);
    state.selection_started.set(selection_started);
    state.number_pick_mode.set(false);
}

pub(super) fn manual_state_visible_candidates(state: &ManualTypingState) -> (Vec<String>, HashMap<usize, Vec<String>>) {
    if state.is_complete() {
        let mut hints = HashMap::new();
        hints.insert(0, vec!["complete".to_owned()]);
        return (vec![state.composed_text.clone()], hints);
    }

    let filtered = manual_filtered_candidates(state);

    let mut hints = HashMap::new();
    let items = filtered
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let label = if candidate.roman_span.is_empty() {
                format!("{} · manual", candidate.kind.label())
            } else {
                format!("{} · {}", candidate.kind.label(), candidate.roman_span)
            };
            hints.insert(index, vec![label]);
            candidate.display_text.clone()
        })
        .collect::<Vec<_>>();
    (items, hints)
}

pub(super) fn refresh_manual_state_candidates(state: &mut ManualTypingState) {
    let remaining = state.remaining_roman();
    if remaining.is_empty() {
        state.candidates.clear();
        state.active_span = None;
        return;
    }

    state.candidates = suggest_manual_character_candidates(&remaining, state.expected_kind, 48);
    ensure_manual_kind_filter(state);
    state.active_span = state.candidates.first().map(|candidate| {
        let span_len = char_len(&candidate.roman_span);
        state.consumed..state.consumed + span_len
    });
}

fn manual_candidate_count(state: &ManualTypingState, kind: ManualComposeKind) -> usize {
    state
        .candidates
        .iter()
        .filter(|candidate| candidate.kind == kind)
        .count()
}

fn manual_kind_order() -> [ManualComposeKind; 3] {
    [
        ManualComposeKind::BaseConsonant,
        ManualComposeKind::Vowel,
        ManualComposeKind::Subscript,
    ]
}

fn ensure_manual_kind_filter(state: &mut ManualTypingState) {
    if manual_candidate_count(state, state.kind_filter) > 0 {
        return;
    }
    if manual_candidate_count(state, state.expected_kind) > 0 {
        state.kind_filter = state.expected_kind;
        return;
    }
    for kind in manual_kind_order() {
        if manual_candidate_count(state, kind) > 0 {
            state.kind_filter = kind;
            return;
        }
    }
}

pub(crate) fn set_manual_kind_filter(kind: ManualComposeKind, state: EditorSignals) -> bool {
    let Some(mut manual) = state.manual_typing_state() else {
        return false;
    };
    if manual_candidate_count(&manual, kind) == 0 {
        return false;
    }
    manual.kind_filter = kind;
    apply_manual_candidates(state, manual, false);
    true
}

pub(crate) fn skip_manual_roman_char(state: EditorSignals) -> bool {
    let Some(mut manual) = state.manual_typing_state() else {
        return false;
    };
    let total = char_len(&manual.raw_roman);
    if manual.consumed >= total {
        return false;
    }
    manual.checkpoints.push(manual.checkpoint());
    manual.consumed = (manual.consumed + 1).min(total);
    refresh_manual_state_candidates(&mut manual);
    apply_manual_candidates(state, manual, false);
    true
}

fn select_manual_candidate(candidate_index: usize, state: EditorSignals) -> bool {
    let Some(mut manual) = state.manual_typing_state() else {
        return false;
    };
    if manual.is_complete() {
        return false;
    }
    let Some(candidate) = manual_filtered_candidates(&manual).get(candidate_index).cloned() else {
        return false;
    };
    manual.checkpoints.push(manual.checkpoint());
    let consumed_len = char_len(&candidate.roman_span);

    manual.consumed = (manual.consumed + consumed_len).min(char_len(&manual.raw_roman));
    manual.composed_text.push_str(&candidate.insert_text);
    manual.expected_kind = match candidate.kind {
        ManualComposeKind::BaseConsonant => ManualComposeKind::Subscript,
        ManualComposeKind::Vowel => ManualComposeKind::BaseConsonant,
        ManualComposeKind::Subscript => ManualComposeKind::Vowel,
    };
    manual.kind_filter = manual.expected_kind;
    refresh_manual_state_candidates(&mut manual);
    apply_manual_candidates(state, manual, true);
    true
}

async fn finalize_manual_selection(typed_space: bool, mut state: EditorSignals) {
    let Some(manual) = state.manual_typing_state() else {
        return;
    };
    if !manual.is_complete() {
        return;
    }
    let choice = manual.composed_text.clone();
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

    let request = ManualSaveRequest {
        roman: manual.raw_roman.clone(),
        khmer: choice,
    };
    save_editor_text(&applied.text);
    state.text.set(applied.text);
    state.clear_candidate_state();
    state.manual_save_request.set(Some(request));
    state.pending_caret.set(Some(applied.caret));
}

pub(crate) async fn commit_manual_selection(typed_space: bool, state: EditorSignals) {
    let Some(manual) = state.manual_typing_state() else {
        return;
    };
    if manual.is_complete() {
        finalize_manual_selection(typed_space, state).await;
        return;
    }

    if !select_manual_candidate(state.selected(), state) {
        return;
    }
    if state
        .manual_typing_state()
        .as_ref()
        .map(ManualTypingState::is_complete)
        .unwrap_or(false)
    {
        finalize_manual_selection(typed_space, state).await;
    }
}

pub(crate) fn save_manual_save_request(mut state: EditorSignals) -> bool {
    let Some(request) = state.manual_save_request() else {
        return false;
    };
    let key = normalize_user_dictionary_key(&request.roman);
    if key.is_empty() {
        state.manual_save_request.set(None);
        return false;
    }

    let mut dictionary = state.user_dictionary();
    let values = dictionary.entry(key).or_default();
    if !values.iter().any(|value| value == &request.khmer) {
        values.insert(0, request.khmer.clone());
    }
    save_user_dictionary(&dictionary);
    state.user_dictionary.set(dictionary);
    state.manual_save_request.set(None);
    true
}

pub(crate) fn dismiss_manual_save_request(mut state: EditorSignals) {
    state.manual_save_request.set(None);
}

pub(crate) fn remove_user_dictionary_mapping(roman: &str, khmer: &str, mut state: EditorSignals) -> bool {
    let key = normalize_user_dictionary_key(roman);
    if key.is_empty() || khmer.trim().is_empty() {
        return false;
    }

    let mut dictionary = state.user_dictionary();
    let mut changed = false;
    if let Some(values) = dictionary.get_mut(&key) {
        let before = values.len();
        values.retain(|value| value != khmer);
        changed = values.len() != before;
        if values.is_empty() {
            dictionary.remove(&key);
        }
    }
    if !changed {
        return false;
    }

    save_user_dictionary(&dictionary);
    state.user_dictionary.set(dictionary);

    if let Some(request) = state.manual_save_request() {
        if normalize_user_dictionary_key(&request.roman) == key && request.khmer == khmer {
            state.manual_save_request.set(None);
        }
    }

    if state.roman_enabled() {
        spawn(update_candidates(state.text(), state));
    }
    true
}

pub(crate) fn undo_manual_step(state: EditorSignals) -> bool {
    let Some(mut manual) = state.manual_typing_state() else {
        return false;
    };
    let Some(checkpoint) = manual.checkpoints.pop() else {
        return false;
    };
    manual.restore_checkpoint(checkpoint);
    refresh_manual_state_candidates(&mut manual);
    apply_manual_candidates(state, manual, false);
    true
}
