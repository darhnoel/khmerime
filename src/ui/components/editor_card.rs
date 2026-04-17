use dioxus::html::Modifiers;
use dioxus::prelude::*;
use roman_lookup::ManualComposeKind;

use crate::ui::editor::{
    click_candidate, commit_active_selection, composition_preview_style, composition_style, is_space_key,
    move_segment_focus, popup_style, segmented_composition_preview_style, segmented_preview_parts,
    select_segment_candidate, set_manual_kind_filter, shortcut_index, shortcut_label, should_exit_number_pick,
    skip_manual_roman_char, update_candidates, visible_page_start, EditorSignals, InputMode, SegmentedSession,
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

fn roman_hint_label(variants: &[String]) -> String {
    // Show all variants if there are 3 or fewer, otherwise show the first 3 followed by ellipsis.
    format!("{}", variants.join(" / "))
}

#[component]
pub(crate) fn EditorCard(state: EditorSignals, font_size: Signal<usize>) -> Element {
    let text_value = state.text();
    let suggestions = state.suggestions();
    let suggestion_total = suggestions.len();
    let page_start = visible_page_start(state.selected(), suggestion_total);
    let recommended_indices = state.recommended_indices();
    let roman_variant_hints = state.roman_variant_hints();
    let has_suggestions = !suggestions.is_empty();
    let manual_state = if state.input_mode() == InputMode::ManualCharacterTyping {
        state.manual_typing_state()
    } else {
        None
    };
    let manual_consonant_count = manual_state
        .as_ref()
        .map(|manual| {
            manual
                .candidates
                .iter()
                .filter(|candidate| candidate.kind == ManualComposeKind::BaseConsonant)
                .count()
        })
        .unwrap_or(0);
    let manual_vowel_count = manual_state
        .as_ref()
        .map(|manual| {
            manual
                .candidates
                .iter()
                .filter(|candidate| candidate.kind == ManualComposeKind::Vowel)
                .count()
        })
        .unwrap_or(0);
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
                        state.manual_save_request.set(None);
                        // Start fresh after text changes so the next ArrowDown selects the first
                        // candidate for the current token instead of continuing stale selection.
                        state.number_pick_mode.set(false);
                        state.selection_started.set(false);
                        state.selected.set(0);
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
                            "Tab" if state.input_mode() == InputMode::ManualCharacterTyping => {
                                event.prevent_default();
                                let Some(manual) = state.manual_typing_state() else {
                                    return;
                                };
                                let target = if manual.kind_filter == ManualComposeKind::BaseConsonant {
                                    ManualComposeKind::Vowel
                                } else {
                                    ManualComposeKind::BaseConsonant
                                };
                                let _ = set_manual_kind_filter(target, state);
                                state.number_pick_mode.set(false);
                            }
                            key
                                if state.input_mode() == InputMode::ManualCharacterTyping
                                    && key.eq_ignore_ascii_case("s")
                                    && modifiers.is_empty() =>
                            {
                                event.prevent_default();
                                let _ = skip_manual_roman_char(state);
                            }
                            "Tab" if !suggestions.is_empty() => {
                                event.prevent_default();
                                let len = suggestions.len();
                                let next = (state.selected() + 1) % len;
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(next, state);
                                } else {
                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(false);
                            }
                            "ArrowDown" if !suggestions.is_empty() => {
                                event.prevent_default();
                                if event.is_auto_repeating() {
                                    return;
                                }
                                if state.segmented_refine_mode() {
                                    let Some(session) = state.segmented_session() else {
                                        return;
                                    };

                                    let len = session.current_candidate_len();
                                    if len == 0 {
                                        return;
                                    }

                                    let next = if !state.selection_started() {
                                        0
                                    } else {
                                        (state.selected() + 1) % len
                                    };

                                    select_segment_candidate(next, state);
                                    state.selection_started.set(true);
                                } else {
                                    let len = suggestions.len();
                                    if len == 0 {
                                        return;
                                    }

                                    let next = if !state.selection_started() {
                                        0
                                    } else {
                                        (state.selected() + 1) % len
                                    };

                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }

                                state.number_pick_mode.set(false);
                                // if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                //     let next = if !state.selection_started() { 0 } else { (state.selected() + 1) % len };
                                //     select_segment_candidate(next, state);
                                // } else {
                                //     if !state.selection_started() {
                                //         state.selected.set(0);
                                //     } else {
                                //         state.selected.set((state.selected() + 1) % len);
                                //     }
                                //     state.selection_started.set(true);
                                // }
                                // state.number_pick_mode.set(false);
                            }
                            "ArrowUp" if !suggestions.is_empty() => {
                                if event.is_auto_repeating() {
                                    return;
                                }
                                event.prevent_default();
                                let len = suggestions.len();
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
                            key if is_space_key(key) && modifiers.contains(Modifiers::SHIFT) && !suggestions.is_empty() => {
                                event.prevent_default();
                                spawn(commit_active_selection(false, state));
                            }
                            key if is_space_key(key) && !suggestions.is_empty() && !state.selection_started() => {
                                event.prevent_default();
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(0, state);
                                } else {
                                    state.selected.set(0);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(true);
                            }
                            key if is_space_key(key) && !suggestions.is_empty() => {
                                event.prevent_default();
                                let len = suggestions.len();
                                let next = (state.selected() + 1) % len;
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(next, state);
                                } else {
                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(true);
                            }
                            "Enter" if !suggestions.is_empty() => {
                                event.prevent_default();
                                spawn(commit_active_selection(false, state));
                            }
                            "Enter" if state.input_mode() == InputMode::ManualCharacterTyping => {
                                event.prevent_default();
                                spawn(commit_active_selection(false, state));
                            }
                            key if state.number_pick_mode() && !suggestions.is_empty() => {
                                if let Some(offset) = shortcut_index(key) {
                                    let page_start = visible_page_start(state.selected(), suggestions.len());
                                    let index = page_start + offset;
                                    if index < suggestions.len() {
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
                if has_suggestions {
                    div {
                        class: "suggestion-popup",
                        "data-testid": "suggestion-popup",
                        style: popup_style(state.popup()),
                        div { class: "candidate-track candidate-track-popup",
                            ul { class: "suggestion-list candidate-list",
                                for (index, item) in suggestions.iter()
                                    .enumerate()
                                    .skip(page_start)
                                    .take(VISIBLE_SUGGESTIONS) {
                                    li {
                                        key: "popup-{index}-{item}",
                                        class: if state.selection_started() && index == state.selected() { "suggestion active" } else { "suggestion" },
                                        button {
                                            onclick: move |_| {
                                                spawn(click_candidate(index, state));
                                            },
                                            span { class: "suggestion-rank", "{shortcut_label(index)}" }
                                            span { class: "suggestion-main",
                                                span { class: "suggestion-word", "{item}" }
                                                if let Some(variants) = roman_variant_hints.get(&index) {
                                                    span { class: "suggestion-roman-hint", "{roman_hint_label(variants)}" }
                                                } else {
                                                    span { class: "suggestion-roman-hint", "(derived)"}
                                                }
                                            }
                                            if recommended_indices.contains(&index) {
                                                span { class: "suggestion-recommended", "គួរប្រើ" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    class: if has_suggestions { "candidate-bar" } else { "candidate-bar candidate-bar-empty" },
                    div { class: "candidate-track candidate-track-mobile",
                        if has_suggestions {
                            ul { class: "suggestion-list candidate-list",
                                for (index, item) in suggestions.iter()
                                    .enumerate()
                                    .skip(page_start)
                                    .take(VISIBLE_SUGGESTIONS) {
                                    li {
                                        key: "mobile-{index}-{item}",
                                        class: if state.selection_started() && index == state.selected() { "suggestion active" } else { "suggestion" },
                                        button {
                                            onclick: move |_| {
                                                spawn(click_candidate(index, state));
                                            },
                                            span { class: "suggestion-rank", "{shortcut_label(index)}" }
                                            span { class: "suggestion-main",
                                                span { class: "suggestion-word", "{item}" }
                                                if let Some(variants) = roman_variant_hints.get(&index) {
                                                    span { class: "suggestion-roman-hint", "{roman_hint_label(variants)}" }
                                                } else {
                                                    span { class: "suggestion-roman-hint", "(derived)"}
                                                }
                                            }
                                            if recommended_indices.contains(&index) {
                                                span { class: "suggestion-recommended", "គួរប្រើ" }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "candidate-empty", aria_hidden: "true",
                                span { class: "segment-placeholder-chip segment-placeholder-chip-1" }
                                span { class: "segment-placeholder-chip segment-placeholder-chip-2" }
                                span { class: "segment-placeholder-chip segment-placeholder-chip-3" }
                            }
                        }
                    }
                    div { class: "candidate-footer",
                        if let Some(manual) = &manual_state {
                            div { class: "manual-kind-switch",
                                button {
                                    class: if manual.kind_filter == ManualComposeKind::BaseConsonant {
                                        "manual-kind-tab active"
                                    } else {
                                        "manual-kind-tab"
                                    },
                                    disabled: manual_consonant_count == 0,
                                    onclick: move |_| {
                                        let _ = set_manual_kind_filter(ManualComposeKind::BaseConsonant, state);
                                    },
                                    "Consonant ({manual_consonant_count})"
                                }
                                button {
                                    class: if manual.kind_filter == ManualComposeKind::Vowel {
                                        "manual-kind-tab active"
                                    } else {
                                        "manual-kind-tab"
                                    },
                                    disabled: manual_vowel_count == 0,
                                    onclick: move |_| {
                                        let _ = set_manual_kind_filter(ManualComposeKind::Vowel, state);
                                    },
                                    "Vowel ({manual_vowel_count})"
                                }
                                button {
                                    class: "manual-kind-tab",
                                    disabled: manual.remaining_roman().is_empty(),
                                    onclick: move |_| {
                                        let _ = skip_manual_roman_char(state);
                                    },
                                    "Skip (S)"
                                }
                            }
                        }
                        div { class: "candidate-hints",
                            span { class: "candidate-hint",
                                span { class: "keycap", "Space" }
                                span { class: "editor-tip-text", "cycle" }
                                span { class: "editor-tip-sep", "or" }
                                span { class: "keycap", "1-5" }
                                span { class: "editor-tip-text", "choose" }
                            }
                            span { class: "candidate-hint",
                                span { class: "keycap", "Enter" }
                                span { class: "editor-tip-sep", "or" }
                                span { class: "keycap", "Shift+Space" }
                                span { class: "editor-tip-text",
                                    if state.input_mode() == InputMode::ManualCharacterTyping {
                                        "pick/finalize"
                                    } else {
                                        "commit"
                                    }
                                }
                            }
                            if state.input_mode() == InputMode::ManualCharacterTyping {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "Tab" }
                                    span { class: "editor-tip-text", "switch kind" }
                                }
                                span { class: "candidate-hint",
                                    span { class: "keycap", "S" }
                                    span { class: "editor-tip-text", "skip 1 roman char" }
                                }
                            }
                            if state.input_mode() != InputMode::ManualCharacterTyping {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "Left/Right" }
                                    span { class: "editor-tip-text", "move segments" }
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
                        if let Some(preview) = suggestions.get(state.selected()).cloned() {
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
            }
        }
    }
}
