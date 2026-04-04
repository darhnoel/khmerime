use std::collections::HashMap;

use dioxus::html::Modifiers;
use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation};

use crate::ui::editor::{
    commit_segmented_selection, commit_selection, composition_preview_style, composition_style, is_space_key,
    move_segment_focus, popup_style, segmented_composition_preview_style, segmented_preview_parts,
    select_segment_candidate, shortcut_index, shortcut_label, should_exit_number_pick, update_candidates,
    visible_page_start, SegmentedSession,
};
use crate::ui::storage::{save_editor_text, save_enabled};
use crate::{CompositionMark, SuggestionPopup, EDITOR_ID, VISIBLE_SUGGESTIONS};

fn render_segmented_composition_preview(
    session: &SegmentedSession,
    mark: &CompositionMark,
    font_size: usize,
) -> Element {
    let (before, focused, after) = segmented_preview_parts(session);
    rsx! {
        div {
            class: "composition-preview composition-preview-segmented",
            style: segmented_composition_preview_style(mark, font_size),
            if !before.is_empty() {
                span { class: "composition-preview-rest", "{before}" }
            }
            span { class: "composition-preview-text", "{focused}" }
            if !after.is_empty() {
                span { class: "composition-preview-rest", "{after}" }
            }
            span { class: "composition-caret", aria_hidden: "true" }
        }
    }
}

#[component]
pub(crate) fn EditorCard(
    engine_ready: Signal<bool>,
    text: Signal<String>,
    roman_enabled: Signal<bool>,
    decoder_mode: Signal<DecoderMode>,
    font_size: Signal<usize>,
    suggestions: Signal<Vec<String>>,
    popup: Signal<Option<SuggestionPopup>>,
    composition: Signal<Option<CompositionMark>>,
    shadow_debug: Signal<Option<ShadowObservation>>,
    segmented_session: Signal<Option<SegmentedSession>>,
    segmented_refine_mode: Signal<bool>,
    active_token: Signal<String>,
    number_pick_mode: Signal<bool>,
    selection_started: Signal<bool>,
    selected: Signal<usize>,
    pending_caret: Signal<Option<usize>>,
    history: Signal<HashMap<String, usize>>,
) -> Element {
    rsx! {
        div { class: "editor-card",
            div { class: "editor-wrap",
                textarea {
                    id: EDITOR_ID,
                    "data-testid": "editor-input",
                    class: if composition().is_some() { "editor editor-composing" } else { "editor" },
                    style: "font-size: {font_size()}px;",
                    value: "{text}",
                    placeholder: "Type roman text here...",
                    spellcheck: "false",
                    autocomplete: "off",
                    autocorrect: "off",
                    oninput: move |event| {
                        let value = event.value();
                        save_editor_text(&value);
                        text.set(value.clone());
                        spawn(update_candidates(value, text, roman_enabled, decoder_mode(), engine_ready, suggestions, popup, composition, shadow_debug, segmented_session, segmented_refine_mode, active_token, number_pick_mode, selection_started, selected, history));
                    },
                    onkeydown: move |event| {
                        let key = event.key().to_string();
                        let modifiers = event.modifiers();

                        if modifiers.contains(Modifiers::ALT)
                            && modifiers.contains(Modifiers::CONTROL)
                            && key.eq_ignore_ascii_case("k")
                        {
                            event.prevent_default();
                            let next = !roman_enabled();
                            roman_enabled.set(next);
                            save_enabled(next);
                            if !next {
                                suggestions.set(Vec::new());
                                popup.set(None);
                                composition.set(None);
                                shadow_debug.set(None);
                                segmented_session.set(None);
                                segmented_refine_mode.set(false);
                                active_token.set(String::new());
                                number_pick_mode.set(false);
                                selection_started.set(false);
                                selected.set(0);
                            } else {
                                spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), engine_ready, suggestions, popup, composition, shadow_debug, segmented_session, segmented_refine_mode, active_token, number_pick_mode, selection_started, selected, history));
                            }
                            return;
                        }

                        if !roman_enabled() {
                            return;
                        }

                        match key.as_str() {
                            "ArrowLeft" if segmented_session().is_some() => {
                                if move_segment_focus(-1, segmented_session, segmented_refine_mode, suggestions, selected, selection_started) {
                                    event.prevent_default();
                                }
                            }
                            "ArrowRight" if segmented_session().is_some() => {
                                if move_segment_focus(1, segmented_session, segmented_refine_mode, suggestions, selected, selection_started) {
                                    event.prevent_default();
                                }
                            }
                            "Tab" if !suggestions().is_empty() => {
                                event.prevent_default();
                                let len = suggestions().len();
                                let next = (selected() + 1) % len;
                                if segmented_refine_mode() && segmented_session().is_some() {
                                    select_segment_candidate(next, segmented_session, segmented_refine_mode, suggestions, selected, selection_started);
                                } else {
                                    selected.set(next);
                                    selection_started.set(true);
                                }
                                number_pick_mode.set(true);
                            }
                            "ArrowDown" if !suggestions().is_empty() => {
                                event.prevent_default();
                                let len = suggestions().len();
                                if segmented_refine_mode() && segmented_session().is_some() {
                                    let next = if !selection_started() { 0 } else { (selected() + 1) % len };
                                    select_segment_candidate(next, segmented_session, segmented_refine_mode, suggestions, selected, selection_started);
                                } else {
                                    if !selection_started() {
                                        selected.set(0);
                                    } else {
                                        selected.set((selected() + 1) % len);
                                    }
                                    selection_started.set(true);
                                }
                                number_pick_mode.set(true);
                            }
                            "ArrowUp" if !suggestions().is_empty() => {
                                event.prevent_default();
                                let len = suggestions().len();
                                if segmented_refine_mode() && segmented_session().is_some() {
                                    let next = if !selection_started() {
                                        len.saturating_sub(1)
                                    } else {
                                        (selected() + len - 1) % len
                                    };
                                    select_segment_candidate(next, segmented_session, segmented_refine_mode, suggestions, selected, selection_started);
                                } else {
                                    if !selection_started() {
                                        selected.set(len.saturating_sub(1));
                                    } else {
                                        selected.set((selected() + len - 1) % len);
                                    }
                                    selection_started.set(true);
                                }
                                number_pick_mode.set(true);
                            }
                            key if is_space_key(key) && modifiers.contains(Modifiers::SHIFT) && !suggestions().is_empty() => {
                                event.prevent_default();
                                if segmented_refine_mode() && segmented_session().is_some() {
                                    spawn(commit_segmented_selection(false, text, segmented_session, segmented_refine_mode, suggestions, popup, composition, shadow_debug, active_token, selection_started, selected, pending_caret, history));
                                } else {
                                    spawn(commit_selection(false, text, suggestions, popup, composition, active_token, selection_started, selected, pending_caret, history));
                                }
                            }
                            key if is_space_key(key) && !suggestions().is_empty() && !selection_started() => {
                                event.prevent_default();
                                if segmented_refine_mode() && segmented_session().is_some() {
                                    select_segment_candidate(0, segmented_session, segmented_refine_mode, suggestions, selected, selection_started);
                                } else {
                                    selected.set(0);
                                    selection_started.set(true);
                                }
                                number_pick_mode.set(true);
                            }
                            key if is_space_key(key) && !suggestions().is_empty() => {
                                event.prevent_default();
                                let len = suggestions().len();
                                let next = (selected() + 1) % len;
                                if segmented_refine_mode() && segmented_session().is_some() {
                                    select_segment_candidate(next, segmented_session, segmented_refine_mode, suggestions, selected, selection_started);
                                } else {
                                    selected.set(next);
                                    selection_started.set(true);
                                }
                                number_pick_mode.set(true);
                            }
                            "Enter" if !suggestions().is_empty() => {
                                event.prevent_default();
                                if segmented_refine_mode() && segmented_session().is_some() {
                                    spawn(commit_segmented_selection(false, text, segmented_session, segmented_refine_mode, suggestions, popup, composition, shadow_debug, active_token, selection_started, selected, pending_caret, history));
                                } else {
                                    spawn(commit_selection(false, text, suggestions, popup, composition, active_token, selection_started, selected, pending_caret, history));
                                }
                            }
                            key if number_pick_mode() && !suggestions().is_empty() => {
                                if let Some(offset) = shortcut_index(key) {
                                    let page_start = visible_page_start(selected(), suggestions().len());
                                    let index = page_start + offset;
                                    if index < suggestions().len() {
                                        event.prevent_default();
                                        if segmented_refine_mode() && segmented_session().is_some() {
                                            select_segment_candidate(index, segmented_session, segmented_refine_mode, suggestions, selected, selection_started);
                                        } else {
                                            selected.set(index);
                                            selection_started.set(true);
                                        }
                                    }
                                } else if should_exit_number_pick(key) {
                                    number_pick_mode.set(false);
                                }
                            }
                            _ => {}
                        }
                    },
                }
                if let Some(session) = segmented_session() {
                    if session.segments.len() > 1 {
                        div { class: "segment-preview", "data-testid": "segment-preview",
                            for (index, segment) in session.segments.iter().enumerate() {
                                button {
                                    key: "{index}-{segment.input}",
                                    class: if segmented_refine_mode() && index == session.focused { "segment-chip active" } else { "segment-chip" },
                                    onclick: move |_| {
                                        if move_segment_focus(index as isize - session.focused as isize, segmented_session, segmented_refine_mode, suggestions, selected, selection_started) {
                                            number_pick_mode.set(true);
                                        }
                                    },
                                    span { class: "segment-chip-output", "{segment.selected_text()}" }
                                    span { class: "segment-chip-input", "{segment.input}" }
                                }
                            }
                        }
                    }
                }
                if let Some(mark) = composition() {
                    if segmented_refine_mode() {
                        if let Some(session) = segmented_session() {
                            {render_segmented_composition_preview(&session, &mark, font_size())}
                        }
                    } else if selection_started() {
                        if let Some(preview) = suggestions().get(selected()).cloned() {
                            div {
                                class: "composition-preview",
                                style: composition_preview_style(&mark, font_size()),
                                span { class: "composition-preview-text", "{preview}" }
                                span { class: "composition-caret", aria_hidden: "true" }
                            }
                        }
                    } else {
                        div {
                            class: "composition-mark",
                            style: composition_style(&mark, false),
                        }
                    }
                }
                if !suggestions().is_empty() {
                    div {
                        class: "suggestion-popup",
                        "data-testid": "suggestion-popup",
                        style: popup_style(popup()),
                        div { class: "suggestion-popup-head", "Suggestions" }
                        ul { class: "suggestion-list",
                            for (index, item) in suggestions()
                                .iter()
                                .enumerate()
                                .skip(visible_page_start(selected(), suggestions().len()))
                                .take(VISIBLE_SUGGESTIONS) {
                                li {
                                    key: "{index}-{item}",
                                    class: if selection_started() && index == selected() { "suggestion active" } else { "suggestion" },
                                    button {
                                        onclick: move |_| {
                                            if segmented_refine_mode() && segmented_session().is_some() {
                                                select_segment_candidate(index, segmented_session, segmented_refine_mode, suggestions, selected, selection_started);
                                                number_pick_mode.set(true);
                                            } else {
                                                selected.set(index);
                                                selection_started.set(true);
                                                spawn(commit_selection(false, text, suggestions, popup, composition, active_token, selection_started, selected, pending_caret, history));
                                            }
                                        },
                                        span { class: "suggestion-rank", "{shortcut_label(index)}" }
                                        span { class: "suggestion-word", "{item}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "editor-note",
                div { class: "editor-note-title", "Quick Keys" }
                div { class: "editor-note-items",
                    div { class: "editor-tip",
                        span { class: "keycap", "Space" }
                        span { class: "editor-tip-text", "cycle" }
                        span { class: "editor-tip-sep", "or" }
                        span { class: "keycap", "1-5" }
                        span { class: "editor-tip-text", "choose" }
                    }
                    div { class: "editor-tip",
                        span { class: "keycap", "Enter" }
                        span { class: "editor-tip-sep", "or" }
                        span { class: "keycap", "Shift+Space" }
                        span { class: "editor-tip-text", "commit" }
                    }
                    div { class: "editor-tip",
                        span { class: "keycap", "Left/Right" }
                        span { class: "editor-tip-text", "move between phrase segments" }
                    }
                }
            }
        }
    }
}
