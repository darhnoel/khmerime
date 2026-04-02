use std::collections::HashMap;

use dioxus::html::Modifiers;
use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation};

use crate::ui::editor::{
    commit_selection, composition_preview_style, composition_style, is_space_key, popup_style, shortcut_index,
    shortcut_label, should_exit_number_pick, update_candidates, visible_page_start,
};
use crate::ui::storage::{save_editor_text, save_enabled};
use crate::{CompositionMark, SuggestionPopup, EDITOR_ID, VISIBLE_SUGGESTIONS};

#[component]
pub(crate) fn EditorCard(
    text: Signal<String>,
    roman_enabled: Signal<bool>,
    decoder_mode: Signal<DecoderMode>,
    font_size: Signal<usize>,
    suggestions: Signal<Vec<String>>,
    popup: Signal<Option<SuggestionPopup>>,
    composition: Signal<Option<CompositionMark>>,
    shadow_debug: Signal<Option<ShadowObservation>>,
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
                    class: "editor",
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
                        spawn(update_candidates(value, text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, active_token, number_pick_mode, selection_started, selected, history));
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
                                active_token.set(String::new());
                                number_pick_mode.set(false);
                                selection_started.set(false);
                                selected.set(0);
                            } else {
                                spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, active_token, number_pick_mode, selection_started, selected, history));
                            }
                            return;
                        }

                        if !roman_enabled() {
                            return;
                        }

                        match key.as_str() {
                            "Tab" if !suggestions().is_empty() => {
                                event.prevent_default();
                                let len = suggestions().len();
                                selected.set((selected() + 1) % len);
                                number_pick_mode.set(true);
                                selection_started.set(true);
                            }
                            "ArrowDown" if !suggestions().is_empty() => {
                                event.prevent_default();
                                let len = suggestions().len();
                                if !selection_started() {
                                    selected.set(0);
                                } else {
                                    selected.set((selected() + 1) % len);
                                }
                                number_pick_mode.set(true);
                                selection_started.set(true);
                            }
                            "ArrowUp" if !suggestions().is_empty() => {
                                event.prevent_default();
                                let len = suggestions().len();
                                if !selection_started() {
                                    selected.set(len.saturating_sub(1));
                                } else {
                                    selected.set((selected() + len - 1) % len);
                                }
                                number_pick_mode.set(true);
                                selection_started.set(true);
                            }
                            key if is_space_key(key) && modifiers.contains(Modifiers::SHIFT) && !suggestions().is_empty() => {
                                event.prevent_default();
                                spawn(commit_selection(false, text, suggestions, popup, composition, active_token, selection_started, selected, pending_caret, history));
                            }
                            key if is_space_key(key) && !suggestions().is_empty() && !selection_started() => {
                                event.prevent_default();
                                selected.set(0);
                                number_pick_mode.set(true);
                                selection_started.set(true);
                            }
                            key if is_space_key(key) && !suggestions().is_empty() => {
                                event.prevent_default();
                                let len = suggestions().len();
                                selected.set((selected() + 1) % len);
                                number_pick_mode.set(true);
                                selection_started.set(true);
                            }
                            "Enter" if !suggestions().is_empty() => {
                                event.prevent_default();
                                spawn(commit_selection(false, text, suggestions, popup, composition, active_token, selection_started, selected, pending_caret, history));
                            }
                            key if number_pick_mode() && !suggestions().is_empty() => {
                                if let Some(offset) = shortcut_index(key) {
                                    let page_start = visible_page_start(selected(), suggestions().len());
                                    let index = page_start + offset;
                                    if index < suggestions().len() {
                                        event.prevent_default();
                                        selected.set(index);
                                        selection_started.set(true);
                                    }
                                } else if should_exit_number_pick(key) {
                                    number_pick_mode.set(false);
                                }
                            }
                            _ => {}
                        }
                    },
                    onkeyup: move |event| {
                        let key = event.key().to_string();
                        if key == "Tab"
                            || key == "ArrowUp"
                            || key == "ArrowDown"
                            || key == "Enter"
                            || is_space_key(&key)
                            || (number_pick_mode() && shortcut_index(&key).is_some())
                        {
                            return;
                        }
                        if roman_enabled() {
                            spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, active_token, number_pick_mode, selection_started, selected, history));
                        }
                    }
                }
                if let Some(mark) = composition() {
                    if selection_started() {
                        if let Some(preview) = suggestions().get(selected()).cloned() {
                            div {
                                class: "composition-preview",
                                style: composition_preview_style(&mark, font_size()),
                                span { class: "composition-preview-text", "{preview}" }
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
                                            selected.set(index);
                                            selection_started.set(true);
                                            spawn(commit_selection(false, text, suggestions, popup, composition, active_token, selection_started, selected, pending_caret, history));
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
            p { class: "editor-note", "Type with roman letters, press Space to cycle or 1-5 to choose, then press Enter or Shift+Space to commit the current word." }
        }
    }
}
