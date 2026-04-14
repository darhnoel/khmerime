use dioxus::html::Modifiers;
use dioxus::prelude::*;

use crate::ui::editor::{
    commit_segmented_selection, commit_selection, composition_preview_style, composition_style, is_space_key,
    move_segment_focus, popup_style, segmented_composition_preview_style, segmented_preview_parts,
    select_segment_candidate, shortcut_index, shortcut_label, should_exit_number_pick, update_candidates,
    visible_page_start, EditorSignals, SegmentedSession,
};
use crate::ui::storage::{save_editor_text, save_enabled};
use crate::{CompositionMark, EDITOR_ID, VISIBLE_SUGGESTIONS};

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
pub(crate) fn EditorCard(state: EditorSignals, font_size: Signal<usize>) -> Element {
    let text_value = state.text();
    rsx! {
        div { class: "editor-card",
            div { class: "editor-wrap",
                textarea {
                    id: EDITOR_ID,
                    "data-testid": "editor-input",
                    class: if state.composition().is_some() { "editor editor-composing" } else { "editor" },
                    style: "font-size: {font_size()}px;",
                    value: "{text_value}",
                    placeholder: "Type roman text here...",
                    spellcheck: "false",
                    autocomplete: "off",
                    autocorrect: "off",
                    oninput: move |event| {
                        let value = event.value();
                        save_editor_text(&value);
                        state.text.set(value.clone());
                        spawn(update_candidates(value, state));
                    },
                    onkeydown: move |event| {
                        let key = event.key().to_string();
                        let modifiers = event.modifiers();

                        if modifiers.contains(Modifiers::ALT)
                            && modifiers.contains(Modifiers::CONTROL)
                            && key.eq_ignore_ascii_case("k")
                        {
                            event.prevent_default();
                            let next = !state.roman_enabled();
                            state.roman_enabled.set(next);
                            save_enabled(next);
                            if !next {
                                state.clear_candidate_state_and_picker();
                            } else {
                                spawn(update_candidates(state.text(), state));
                            }
                            return;
                        }

                        if !state.roman_enabled() {
                            return;
                        }

                        match key.as_str() {
                            "ArrowLeft" if state.segmented_session().is_some() => {
                                if move_segment_focus(-1, state) {
                                    event.prevent_default();
                                }
                            }
                            "ArrowRight" if state.segmented_session().is_some() => {
                                if move_segment_focus(1, state) {
                                    event.prevent_default();
                                }
                            }
                            "Tab" if !state.suggestions().is_empty() => {
                                event.prevent_default();
                                let len = state.suggestions().len();
                                let next = (state.selected() + 1) % len;
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(next, state);
                                } else {
                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(false);
                            }
                            "ArrowDown" if !state.suggestions().is_empty() => {
                                event.prevent_default();
                                let len = state.suggestions().len();
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    let next = if !state.selection_started() { 0 } else { (state.selected() + 1) % len };
                                    select_segment_candidate(next, state);
                                } else {
                                    if !state.selection_started() {
                                        state.selected.set(0);
                                    } else {
                                        state.selected.set((state.selected() + 1) % len);
                                    }
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(false);
                            }
                            "ArrowUp" if !state.suggestions().is_empty() => {
                                event.prevent_default();
                                let len = state.suggestions().len();
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    let next = if !state.selection_started() {
                                        len.saturating_sub(1)
                                    } else {
                                        (state.selected() + len - 1) % len
                                    };
                                    select_segment_candidate(next, state);
                                } else {
                                    if !state.selection_started() {
                                        state.selected.set(len.saturating_sub(1));
                                    } else {
                                        state.selected.set((state.selected() + len - 1) % len);
                                    }
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(false);
                            }
                            key if is_space_key(key) && modifiers.contains(Modifiers::SHIFT) && !state.suggestions().is_empty() => {
                                event.prevent_default();
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    spawn(commit_segmented_selection(false, state));
                                } else {
                                    spawn(commit_selection(false, state));
                                }
                            }
                            key if is_space_key(key) && !state.suggestions().is_empty() && !state.selection_started() => {
                                event.prevent_default();
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(0, state);
                                } else {
                                    state.selected.set(0);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(true);
                            }
                            key if is_space_key(key) && !state.suggestions().is_empty() => {
                                event.prevent_default();
                                let len = state.suggestions().len();
                                let next = (state.selected() + 1) % len;
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(next, state);
                                } else {
                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(true);
                            }
                            "Enter" if !state.suggestions().is_empty() => {
                                event.prevent_default();
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    spawn(commit_segmented_selection(false, state));
                                } else {
                                    spawn(commit_selection(false, state));
                                }
                            }
                            key if state.number_pick_mode() && !state.suggestions().is_empty() => {
                                if let Some(offset) = shortcut_index(key) {
                                    let page_start = visible_page_start(state.selected(), state.suggestions().len());
                                    let index = page_start + offset;
                                    if index < state.suggestions().len() {
                                        event.prevent_default();
                                        if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                            select_segment_candidate(index, state);
                                        } else {
                                            state.selected.set(index);
                                            state.selection_started.set(true);
                                        }
                                    }
                                } else if should_exit_number_pick(key) {
                                    state.number_pick_mode.set(false);
                                }
                            }
                            _ => {}
                        }
                    },
                }
                if let Some(session) = state.segmented_session() {
                    if session.segments.len() > 1 {
                        div { class: "segment-preview", "data-testid": "segment-preview",
                            for (index, segment) in session.segments.iter().enumerate() {
                                button {
                                    key: "{index}-{segment.input}",
                                    class: if state.segmented_refine_mode() && index == session.focused { "segment-chip active" } else { "segment-chip" },
                                    onclick: move |_| {
                                        if move_segment_focus(index as isize - session.focused as isize, state) {
                                            state.number_pick_mode.set(false);
                                        }
                                    },
                                    span { class: "segment-chip-output", "{segment.selected_text()}" }
                                    span { class: "segment-chip-input", "{segment.input}" }
                                }
                            }
                        }
                    }
                }
                if let Some(mark) = state.composition() {
                    if state.segmented_refine_mode() {
                        if let Some(session) = state.segmented_session() {
                            {render_segmented_composition_preview(&session, &mark, font_size())}
                        }
                    } else if state.selection_started() {
                        if let Some(preview) = state.suggestions().get(state.selected()).cloned() {
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
                if !state.suggestions().is_empty() {
                    div {
                        class: "suggestion-popup",
                        "data-testid": "suggestion-popup",
                        style: popup_style(state.popup()),
                        div { class: "suggestion-popup-head", "Suggestions" }
                        ul { class: "suggestion-list",
                            for (index, item) in state.suggestions()
                                .iter()
                                .enumerate()
                                .skip(visible_page_start(state.selected(), state.suggestions().len()))
                                .take(VISIBLE_SUGGESTIONS) {
                                li {
                                    key: "{index}-{item}",
                                    class: if state.selection_started() && index == state.selected() { "suggestion active" } else { "suggestion" },
                                    button {
                                        onclick: move |_| {
                                            if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                                select_segment_candidate(index, state);
                                                state.number_pick_mode.set(false);
                                            } else {
                                                state.selected.set(index);
                                                state.selection_started.set(true);
                                                spawn(commit_selection(false, state));
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
