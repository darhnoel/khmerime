use dioxus::prelude::*;

use crate::ui::editor::{update_candidates, EditorSignals};
use crate::ui::storage::{save_editor_text, save_enabled, save_font_size};
use crate::{MAX_FONT_SIZE, MIN_FONT_SIZE};

#[component]
pub(crate) fn AppToolbar(state: EditorSignals, show_guide: Signal<bool>, font_size: Signal<usize>) -> Element {
    rsx! {
        div { class: "workspace-top",
            div { class: "toolbar",
                div { class: "font-tools",
                    span { class: "tool-label", "Font size" }
                    button {
                        class: "tool-button",
                        "data-testid": "font-decrease",
                        onclick: move |_| {
                            let next = font_size().saturating_sub(2).max(MIN_FONT_SIZE);
                            font_size.set(next);
                            save_font_size(next, MIN_FONT_SIZE, MAX_FONT_SIZE);
                            if state.roman_enabled() {
                                spawn(update_candidates(state.text(), state));
                            }
                        },
                        "A-"
                    }
                    div { class: "font-pill", "{font_size()}px" }
                    button {
                        class: "tool-button",
                        "data-testid": "font-increase",
                        onclick: move |_| {
                            let next = (font_size() + 2).min(MAX_FONT_SIZE);
                            font_size.set(next);
                            save_font_size(next, MIN_FONT_SIZE, MAX_FONT_SIZE);
                            if state.roman_enabled() {
                                spawn(update_candidates(state.text(), state));
                            }
                        },
                        "A+"
                    }
                }
                div { class: "mode-tools",
                    button {
                        class: if state.roman_enabled() { "mode-pill active" } else { "mode-pill" },
                        "data-testid": "toggle-live-edit",
                        onclick: move |_| {
                            let next = !state.roman_enabled();
                            state.roman_enabled.set(next);
                            save_enabled(next);
                            if next {
                                spawn(update_candidates(state.text(), state));
                            } else {
                                state.clear_candidate_state_and_picker();
                            }
                        },
                        if state.roman_enabled() {
                            "Live Edit"
                        } else {
                            "Live Edit Off"
                        }
                    }
                    if !state.engine_ready() {
                        div {
                            class: "engine-status loading",
                            "data-testid": "engine-status",
                            span { class: "engine-status-dot", aria_hidden: "true" }
                            span { "Preparing lexicon..." }
                        }
                    }
                    button {
                        class: if show_guide() { "ghost active" } else { "ghost" },
                        "data-testid": "toggle-rules",
                        onclick: move |_| show_guide.set(!show_guide()),
                        if show_guide() {
                            "Hide Rules"
                        } else {
                            "Rules"
                        }
                    }
                    button {
                        class: "ghost",
                        "data-testid": "clear-editor",
                        onclick: move |_| {
                            state.text.set(String::new());
                            save_editor_text("");
                            state.clear_candidate_state_and_picker();
                            state.pending_caret.set(Some(0));
                        },
                        "Clear"
                    }
                }
            }
        }
    }
}
