use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation, Transliterator};

use crate::engine;

use super::manual_flow::{manual_state_visible_candidates, refresh_manual_state_candidates};
use super::segmented_flow::build_segmented_session;
use super::view_helpers::{candidate_composition_mark, suggestion_popup_position};
use super::{slice_chars, EditorSignals, InputMode, SegmentedSession};

const SHADOW_DEBOUNCE_SHORT_MS: u32 = 220;
const SHADOW_DEBOUNCE_MEDIUM_MS: u32 = 320;
const SHADOW_DEBOUNCE_LONG_MS: u32 = 420;

fn cancel_suggestion_loading(mut state: EditorSignals) {
    state.suggestion_loading.set(false);
    state
        .suggestion_request_id
        .set(state.suggestion_request_id().wrapping_add(1));
}

fn begin_shadow_request(mut state: EditorSignals) -> u64 {
    let request_id = state.suggestion_request_id().wrapping_add(1);
    state.suggestion_request_id.set(request_id);
    state.suggestion_loading.set(false);
    request_id
}

fn shadow_request_is_stale(state: EditorSignals, request_id: u64, text: &str, token: &str) -> bool {
    state.suggestion_request_id() != request_id || state.text() != text || state.active_token() != token
}

fn shadow_debounce_ms(token: &str) -> u32 {
    match token.chars().count() {
        0..=7 => SHADOW_DEBOUNCE_SHORT_MS,
        8..=11 => SHADOW_DEBOUNCE_MEDIUM_MS,
        _ => SHADOW_DEBOUNCE_LONG_MS,
    }
}

#[cfg(target_arch = "wasm32")]
async fn shadow_debounce_delay(delay_ms: u32) {
    gloo_timers::future::TimeoutFuture::new(delay_ms).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn shadow_debounce_delay(delay_ms: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(u64::from(delay_ms))).await;
}

fn spawn_shadow_refinement(mut state: EditorSignals, value: String, token: String, legacy_items: Vec<String>) {
    let request_id = begin_shadow_request(state);
    let delay_ms = shadow_debounce_ms(&token);

    let mut state_loading = state;
    let value_loading = value.clone();
    let token_loading = token.clone();
    spawn(async move {
        shadow_debounce_delay(delay_ms).await;
        if state_loading.suggestion_request_id() != request_id {
            return;
        }
        if shadow_request_is_stale(state_loading, request_id, &value_loading, &token_loading) {
            state_loading.suggestion_loading.set(false);
            return;
        }
        state_loading.suggestion_loading.set(true);
    });

    spawn(async move {
        // Keep shadow decode off the hot typing path and only refine after debounce.
        shadow_debounce_delay(delay_ms).await;
        if state.suggestion_request_id() != request_id {
            return;
        }
        if shadow_request_is_stale(state, request_id, &value, &token) {
            state.suggestion_loading.set(false);
            return;
        }
        let history_shadow = state.history();
        let observation = engine(DecoderMode::Shadow).shadow_observation(&token, &history_shadow);
        if shadow_request_is_stale(state, request_id, &value, &token) {
            state.suggestion_loading.set(false);
            return;
        }
        let next_segmented = build_segmented_session(&observation, &token, &history_shadow);
        let visible = choose_visible_suggestions(
            &legacy_items,
            &observation,
            next_segmented.as_ref(),
            state.segmented_refine_mode(),
        );
        let (visible, user_keys) = merge_with_user_dictionary(&token, &state.user_dictionary(), &visible, 15);
        let (recommended_indices, mut roman_variant_hints) =
            recommended_indices_and_roman_hints(engine(DecoderMode::Legacy), &token, &visible);
        decorate_user_dictionary_hints(&visible, &user_keys, &mut roman_variant_hints);

        state.shadow_debug.set(Some(observation));
        state.segmented_session.set(next_segmented);
        state.segmented_refine_mode.set(false);
        state.recommended_indices.set(recommended_indices);
        state.roman_variant_hints.set(roman_variant_hints);
        let preserve_selection = state.active_token() == token && !state.suggestions().is_empty();
        apply_visible_candidates(state, visible, preserve_selection);
        if state.suggestion_request_id() == request_id {
            state.suggestion_loading.set(false);
        }
    });
}

pub(crate) async fn update_candidates(value: String, mut state: EditorSignals) {
    if !state.roman_enabled() {
        cancel_suggestion_loading(state);
        state.clear_candidate_state_and_picker();
        return;
    }

    let live_text = state.text;
    let caret = crate::ui::platform::current_editor_caret()
        .await
        .unwrap_or_else(|| value.chars().count());
    if live_text() != value {
        return;
    }

    let bounds = Transliterator::token_bounds(&value, caret, false);
    let token = slice_chars(&value, bounds.clone());
    if token.trim().is_empty() {
        cancel_suggestion_loading(state);
        state.clear_candidate_state_and_picker();
        return;
    }

    if state.input_mode() == InputMode::ManualCharacterTyping {
        cancel_suggestion_loading(state);
        state.shadow_debug.set(None);
        state.segmented_session.set(None);
        state.segmented_refine_mode.set(false);

        let mut manual_state = match state.manual_typing_state() {
            Some(existing) if existing.raw_roman == token => existing,
            _ => super::ManualTypingState::new(token.clone()),
        };
        refresh_manual_state_candidates(&mut manual_state);
        let (items, roman_variant_hints) = manual_state_visible_candidates(&manual_state);
        let preserve_selection = state.active_token() == token && !state.suggestions().is_empty();
        let popup_position = if items.is_empty() {
            None
        } else {
            suggestion_popup_position(caret).await
        };
        if live_text() != value {
            return;
        }
        let composition_mark = candidate_composition_mark(bounds.start, &token).await;
        if live_text() != value {
            return;
        }
        state.popup.set(popup_position);
        state.composition.set(composition_mark);
        state.active_token.set(token.clone());
        state.manual_typing_state.set(Some(manual_state));
        state.recommended_indices.set(Vec::new());
        state.roman_variant_hints.set(roman_variant_hints);
        apply_visible_candidates(state, items, preserve_selection);
        return;
    }

    if !state.engine_ready() {
        cancel_suggestion_loading(state);
        state.clear_candidate_state_and_picker();
        state.active_token.set(token);
        return;
    }

    let history_snapshot = state.history();
    let legacy = engine(DecoderMode::Legacy);
    let legacy_items = legacy.suggest(&token, &history_snapshot);
    if live_text() != value {
        return;
    }
    let shadow_requested =
        state.engine_full_ready() && state.decoder_mode() == DecoderMode::Shadow && token.chars().count() >= 3;
    if !shadow_requested {
        cancel_suggestion_loading(state);
    }
    state.shadow_debug.set(None);
    state.segmented_session.set(None);
    state.segmented_refine_mode.set(false);
    let (items, user_keys) = merge_with_user_dictionary(&token, &state.user_dictionary(), &legacy_items, 15);
    let (recommended_indices, mut roman_variant_hints) = recommended_indices_and_roman_hints(legacy, &token, &items);
    decorate_user_dictionary_hints(&items, &user_keys, &mut roman_variant_hints);
    let preserve_selection = state.active_token() == token && !state.suggestions().is_empty();
    let popup_position = if items.is_empty() {
        None
    } else {
        suggestion_popup_position(caret).await
    };
    if live_text() != value {
        return;
    }
    let composition_mark = candidate_composition_mark(bounds.start, &token).await;
    if live_text() != value {
        return;
    }
    state.popup.set(popup_position);
    state.composition.set(composition_mark);
    state.active_token.set(token.clone());
    state.recommended_indices.set(recommended_indices);
    state.roman_variant_hints.set(roman_variant_hints);
    apply_visible_candidates(state, items, preserve_selection);

    if shadow_requested {
        spawn_shadow_refinement(state, value, token, legacy_items);
    }
}

fn apply_visible_candidates(mut state: EditorSignals, items: Vec<String>, preserve_selection: bool) {
    if !preserve_selection || items.is_empty() {
        state.number_pick_mode.set(false);
        state.selection_started.set(false);
        state.selected.set(0);
    } else if state.selected() >= items.len() {
        state.selected.set(items.len().saturating_sub(1));
    }
    state.suggestions.set(items);
}

fn merge_with_user_dictionary(
    token: &str,
    user_dictionary: &HashMap<String, Vec<String>>,
    fallback: &[String],
    limit: usize,
) -> (Vec<String>, HashSet<String>) {
    let user_items = user_dictionary_exact_matches(token, user_dictionary);
    let user_keys = user_items
        .iter()
        .map(|item| normalized_suggestion_key(item))
        .collect::<HashSet<_>>();
    (
        normalize_visible_suggestions(merge_suggestion_lists(&user_items, fallback, limit)),
        user_keys,
    )
}

fn decorate_user_dictionary_hints(
    items: &[String],
    user_keys: &HashSet<String>,
    hints: &mut HashMap<usize, Vec<String>>,
) {
    for (index, item) in items.iter().enumerate() {
        if user_keys.contains(&normalized_suggestion_key(item)) {
            let hint = hints.entry(index).or_default();
            if !hint.iter().any(|label| label == "saved") {
                hint.insert(0, "saved".to_owned());
            }
        }
    }
}

fn user_dictionary_exact_matches(token: &str, user_dictionary: &HashMap<String, Vec<String>>) -> Vec<String> {
    let key = normalize_user_dictionary_key(token);
    if key.is_empty() {
        return Vec::new();
    }
    let mut values = user_dictionary.get(&key).cloned().unwrap_or_default();
    values.dedup();
    values
}

pub(super) fn normalize_user_dictionary_key(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                Some(ch.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn choose_visible_suggestions(
    legacy_items: &[String],
    observation: &ShadowObservation,
    segmented_session: Option<&SegmentedSession>,
    segmented_refine_mode: bool,
) -> Vec<String> {
    if segmented_refine_mode {
        if let Some(session) = segmented_session {
            return normalize_visible_suggestions(session.focused_candidates());
        }
    }
    if !observation.wfst_top5.is_empty() {
        normalize_visible_suggestions(merge_suggestion_lists(&observation.wfst_top5, legacy_items, 10))
    } else if let Some(session) = segmented_session {
        normalize_visible_suggestions(session.focused_candidates())
    } else {
        normalize_visible_suggestions(legacy_items.to_vec())
    }
}

pub(super) fn recommended_indices_and_roman_hints(
    legacy: &Transliterator,
    token: &str,
    items: &[String],
) -> (Vec<usize>, HashMap<usize, Vec<String>>) {
    let exact_keys = legacy
        .exact_match_targets(token)
        .into_iter()
        .map(|item| normalized_suggestion_key(&item))
        .collect::<HashSet<_>>();

    let mut indices = Vec::new();
    let mut hints = HashMap::<usize, Vec<String>>::new();
    for (index, item) in items.iter().enumerate() {
        if exact_keys.contains(&normalized_suggestion_key(item)) {
            indices.push(index);
        }
        let mut variants = legacy.exact_match_roman_variants(token, item);
        variants.truncate(3);
        if !variants.is_empty() {
            hints.insert(index, variants);
        }
    }

    (indices, hints)
}

pub(crate) fn normalized_suggestion_key(item: &str) -> String {
    item.chars().filter(|ch| !ch.is_whitespace()).collect()
}

pub(super) fn connect_khmer_display(item: &str) -> String {
    let parts = item.split_whitespace().collect::<Vec<_>>();
    if parts.len() <= 1 {
        return item.to_owned();
    }
    if parts.iter().all(|part| part.chars().any(is_khmer_char)) {
        parts.concat()
    } else {
        item.to_owned()
    }
}

fn is_khmer_char(ch: char) -> bool {
    ('\u{1780}'..='\u{17ff}').contains(&ch) || ('\u{19e0}'..='\u{19ff}').contains(&ch)
}

pub(super) fn normalize_visible_suggestions(items: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    for item in items {
        let display = connect_khmer_display(&item);
        let key = normalized_suggestion_key(&display);
        if seen.insert(key) {
            normalized.push(display);
        }
    }

    normalized
}

fn merge_suggestion_lists(primary: &[String], fallback: &[String], limit: usize) -> Vec<String> {
    let mut merged = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    for item in primary.iter().chain(fallback.iter()) {
        let key = normalized_suggestion_key(item);
        if seen.insert(key) {
            merged.push(item.clone());
            if merged.len() >= limit {
                break;
            }
        }
    }

    merged
}
