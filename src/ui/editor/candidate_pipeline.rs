use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation, Transliterator};

use crate::engine;

use super::manual_flow::{manual_state_visible_candidates, refresh_manual_state_candidates};
use super::segmented_flow::build_segmented_session;
use super::view_helpers::{candidate_composition_mark, suggestion_popup_position};
use super::{slice_chars, EditorSignals, InputMode, SegmentedSession};

pub(crate) async fn update_candidates(value: String, mut state: EditorSignals) {
    if !state.roman_enabled() {
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
        state.clear_candidate_state_and_picker();
        return;
    }

    if state.input_mode() == InputMode::ManualCharacterTyping {
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

    #[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
    if shadow_requested {
        let mut state_shadow = state;
        let value_shadow = value.clone();
        let token_shadow = token.clone();
        let legacy_items_shadow = legacy_items.clone();
        spawn(async move {
            // Debounce heavy shadow decode so typing stays responsive.
            gloo_timers::future::TimeoutFuture::new(120).await;
            if state_shadow.text() != value_shadow {
                return;
            }
            let history_shadow = state_shadow.history();
            let observation = engine(DecoderMode::Shadow).shadow_observation(&token_shadow, &history_shadow);
            if state_shadow.text() != value_shadow {
                return;
            }
            let next_segmented = build_segmented_session(&observation, &token_shadow, &history_shadow);
            let visible = choose_visible_suggestions(
                &legacy_items_shadow,
                &observation,
                next_segmented.as_ref(),
                state_shadow.segmented_refine_mode(),
            );
            let (visible, user_keys) =
                merge_with_user_dictionary(&token_shadow, &state_shadow.user_dictionary(), &visible, 15);
            let (recommended_indices, mut roman_variant_hints) =
                recommended_indices_and_roman_hints(engine(DecoderMode::Legacy), &token_shadow, &visible);
            decorate_user_dictionary_hints(&visible, &user_keys, &mut roman_variant_hints);

            state_shadow.shadow_debug.set(Some(observation));
            state_shadow.segmented_session.set(next_segmented);
            state_shadow.segmented_refine_mode.set(false);
            state_shadow.recommended_indices.set(recommended_indices);
            state_shadow.roman_variant_hints.set(roman_variant_hints);
            let preserve_selection =
                state_shadow.active_token() == token_shadow && !state_shadow.suggestions().is_empty();
            apply_visible_candidates(state_shadow, visible, preserve_selection);
        });
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    if shadow_requested {
        let observation = engine(DecoderMode::Shadow).shadow_observation(&token, &history_snapshot);
        if live_text() != value {
            return;
        }
        let next_segmented = build_segmented_session(&observation, &token, &history_snapshot);
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
