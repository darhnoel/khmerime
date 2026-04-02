use std::collections::HashMap;

use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation, Transliterator};

use crate::ui::platform::{current_editor_caret, editor_composition_mark, editor_popup_position};
use crate::ui::storage::{save_editor_text, save_history};
use crate::{
    engine, CompositionMark, SuggestionPopup, FALLBACK_POPUP_LEFT, FALLBACK_POPUP_TOP,
};

fn slice_chars(input: &str, range: std::ops::Range<usize>) -> String {
    input
        .chars()
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
        .collect()
}

pub(crate) async fn update_candidates(
    value: String,
    live_text: Signal<String>,
    roman_enabled: Signal<bool>,
    decoder_mode: DecoderMode,
    mut suggestions: Signal<Vec<String>>,
    mut popup: Signal<Option<SuggestionPopup>>,
    mut composition: Signal<Option<CompositionMark>>,
    mut shadow_debug: Signal<Option<ShadowObservation>>,
    mut active_token: Signal<String>,
    mut number_pick_mode: Signal<bool>,
    mut selection_started: Signal<bool>,
    mut selected: Signal<usize>,
    history: Signal<HashMap<String, usize>>,
) {
    if !roman_enabled() {
        suggestions.set(Vec::new());
        popup.set(None);
        composition.set(None);
        shadow_debug.set(None);
        active_token.set(String::new());
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
        return;
    }

    let caret = current_editor_caret()
        .await
        .unwrap_or_else(|| value.chars().count());
    if live_text() != value {
        return;
    }

    let bounds = Transliterator::token_bounds(&value, caret, false);
    let token = slice_chars(&value, bounds.clone());
    if token.trim().is_empty() {
        suggestions.set(Vec::new());
        popup.set(None);
        composition.set(None);
        shadow_debug.set(None);
        active_token.set(String::new());
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
        return;
    }

    let legacy = engine(DecoderMode::Legacy);
    let items = legacy.suggest(&token, &history());
    if live_text() != value {
        return;
    }
    if decoder_mode != DecoderMode::Shadow {
        shadow_debug.set(None);
    }
    let preserve_selection = active_token() == token && !suggestions().is_empty();
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
    popup.set(popup_position);
    composition.set(composition_mark);
    active_token.set(token.clone());
    if !preserve_selection {
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
    } else if items.is_empty() {
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
    } else if selected() >= items.len() {
        selected.set(items.len().saturating_sub(1));
    }
    suggestions.set(items);
    if decoder_mode == DecoderMode::Shadow {
        spawn(update_shadow_debug(
            value,
            live_text,
            history,
            shadow_debug,
            active_token,
        ));
    }
}

fn default_popup_position() -> SuggestionPopup {
    SuggestionPopup {
        left: FALLBACK_POPUP_LEFT,
        top: FALLBACK_POPUP_TOP,
    }
}

async fn suggestion_popup_position(caret: usize) -> Option<SuggestionPopup> {
    Some(
        editor_popup_position(caret)
            .await
            .unwrap_or_else(default_popup_position),
    )
}

async fn candidate_composition_mark(start: usize, token: &str) -> Option<CompositionMark> {
    editor_composition_mark(start, token).await
}

async fn update_shadow_debug(
    value: String,
    live_text: Signal<String>,
    history: Signal<HashMap<String, usize>>,
    mut shadow_debug: Signal<Option<ShadowObservation>>,
    active_token: Signal<String>,
) {
    if live_text() != value {
        return;
    }
    let token = active_token();
    if token.trim().is_empty() {
        shadow_debug.set(None);
        return;
    }
    let shadow = engine(DecoderMode::Shadow);
    let observation = shadow.shadow_observation(&token, &history());
    if live_text() != value || active_token() != token {
        return;
    }
    shadow_debug.set(Some(observation));
}

pub(crate) async fn commit_selection(
    typed_space: bool,
    mut text: Signal<String>,
    mut suggestions: Signal<Vec<String>>,
    mut popup: Signal<Option<SuggestionPopup>>,
    mut composition: Signal<Option<CompositionMark>>,
    mut active_token: Signal<String>,
    mut selection_started: Signal<bool>,
    mut selected: Signal<usize>,
    mut pending_caret: Signal<Option<usize>>,
    mut history: Signal<HashMap<String, usize>>,
) {
    let items = suggestions();
    if items.is_empty() {
        return;
    }
    let Some(choice) = items.get(selected()).cloned() else {
        return;
    };
    let current_text = text();
    let caret = current_editor_caret()
        .await
        .unwrap_or_else(|| current_text.chars().count());
    let applied = Transliterator::apply_suggestion(&current_text, caret, &choice, typed_space);

    let mut next_history = history();
    Transliterator::learn(&mut next_history, &choice);
    save_history(&next_history);
    history.set(next_history);

    save_editor_text(&applied.text);
    text.set(applied.text);
    suggestions.set(Vec::new());
    popup.set(None);
    composition.set(None);
    active_token.set(String::new());
    selection_started.set(false);
    selected.set(0);
    pending_caret.set(Some(applied.caret));
}

pub(crate) async fn refresh_popup_position(mut popup: Signal<Option<SuggestionPopup>>) {
    let Some(caret) = current_editor_caret().await else {
        popup.set(None);
        return;
    };
    popup.set(suggestion_popup_position(caret).await);
}

pub(crate) fn popup_style(popup: Option<SuggestionPopup>) -> String {
    let Some(popup) = popup else {
        return "display:none;".to_owned();
    };
    format!("left:{:.1}px; top:{:.1}px;", popup.left, popup.top)
}

pub(crate) fn composition_style(mark: &CompositionMark, selection_started: bool) -> String {
    let top = mark.top + mark.height - 3.0;
    let opacity = if selection_started { 0.75 } else { 1.0 };
    format!(
        "left:{:.1}px; top:{:.1}px; width:{:.1}px; opacity:{:.2};",
        mark.left, top, mark.width, opacity
    )
}

pub(crate) fn composition_preview_style(mark: &CompositionMark, font_size: usize) -> String {
    format!(
        "left:{:.1}px; top:{:.1}px; width:{:.1}px; height:{:.1}px; font-size:{}px;",
        mark.left, mark.top, mark.width, mark.height, font_size
    )
}

pub(crate) fn shortcut_index(key: &str) -> Option<usize> {
    match key {
        "1" => Some(0),
        "2" => Some(1),
        "3" => Some(2),
        "4" => Some(3),
        "5" => Some(4),
        _ => None,
    }
}

pub(crate) fn shortcut_label(index: usize) -> String {
    ((index % crate::VISIBLE_SUGGESTIONS) + 1).to_string()
}

pub(crate) fn visible_page_start(selected: usize, total: usize) -> usize {
    if total <= crate::VISIBLE_SUGGESTIONS {
        0
    } else {
        (selected / crate::VISIBLE_SUGGESTIONS) * crate::VISIBLE_SUGGESTIONS
    }
}

pub(crate) fn should_exit_number_pick(key: &str) -> bool {
    matches!(key, "Backspace" | "Delete" | "ArrowLeft" | "ArrowRight" | "Escape") || key.chars().count() == 1
}

pub(crate) fn is_space_key(key: &str) -> bool {
    matches!(key, " " | "Space" | "Spacebar")
}
