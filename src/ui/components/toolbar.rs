use std::collections::HashMap;

use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation};

use crate::ui::editor::update_candidates;
use crate::ui::storage::{save_decoder_mode, save_editor_text, save_enabled, save_font_size};
use crate::{CompositionMark, SuggestionPopup, MAX_FONT_SIZE, MIN_FONT_SIZE};

#[component]
pub(crate) fn AppToolbar(
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
        div { class: "workspace-top",
            div { class: "hero",
                h1 { "Khmer IME" }
            }
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
                            if roman_enabled() {
                                spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, active_token, number_pick_mode, selection_started, selected, history));
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
                            if roman_enabled() {
                                spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, active_token, number_pick_mode, selection_started, selected, history));
                            }
                        },
                        "A+"
                    }
                }
                div { class: "mode-tools",
                    div { class: "mode-switch",
                        button {
                            class: if roman_enabled() { "mode-pill active" } else { "mode-pill" },
                            "data-testid": "mode-roman",
                            onclick: move |_| {
                                if !roman_enabled() {
                                    roman_enabled.set(true);
                                    save_enabled(true);
                                    spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, active_token, number_pick_mode, selection_started, selected, history));
                                }
                            },
                            "Roman To Khmer"
                        }
                        button {
                            class: if !roman_enabled() { "mode-pill active" } else { "mode-pill" },
                            "data-testid": "mode-raw",
                            onclick: move |_| {
                                if roman_enabled() {
                                    roman_enabled.set(false);
                                    save_enabled(false);
                                    suggestions.set(Vec::new());
                                    popup.set(None);
                                    composition.set(None);
                                    shadow_debug.set(None);
                                    number_pick_mode.set(false);
                                    selection_started.set(false);
                                    selected.set(0);
                                }
                            },
                            "Raw Roman"
                        }
                    }
                    div { class: "decoder-tools",
                        span { class: "tool-label", "Decoder" }
                        div { class: "mode-switch",
                            button {
                                class: if decoder_mode() == DecoderMode::Legacy { "mode-pill active" } else { "mode-pill" },
                                "data-testid": "decoder-legacy",
                                onclick: move |_| {
                                    if decoder_mode() != DecoderMode::Legacy {
                                        decoder_mode.set(DecoderMode::Legacy);
                                        save_decoder_mode(DecoderMode::Legacy);
                                        shadow_debug.set(None);
                                        if roman_enabled() {
                                            spawn(update_candidates(text(), text, roman_enabled, DecoderMode::Legacy, suggestions, popup, composition, shadow_debug, active_token, number_pick_mode, selection_started, selected, history));
                                        }
                                    }
                                },
                                "Legacy"
                            }
                            button {
                                class: if decoder_mode() == DecoderMode::Shadow { "mode-pill active" } else { "mode-pill" },
                                "data-testid": "decoder-shadow",
                                onclick: move |_| {
                                    if decoder_mode() != DecoderMode::Shadow {
                                        decoder_mode.set(DecoderMode::Shadow);
                                        save_decoder_mode(DecoderMode::Shadow);
                                        if roman_enabled() {
                                            spawn(update_candidates(text(), text, roman_enabled, DecoderMode::Shadow, suggestions, popup, composition, shadow_debug, active_token, number_pick_mode, selection_started, selected, history));
                                        }
                                    }
                                },
                                "Shadow"
                            }
                        }
                    }
                    button {
                        class: "ghost",
                        "data-testid": "clear-editor",
                        onclick: move |_| {
                            text.set(String::new());
                            save_editor_text("");
                            suggestions.set(Vec::new());
                            popup.set(None);
                            composition.set(None);
                            shadow_debug.set(None);
                            active_token.set(String::new());
                            number_pick_mode.set(false);
                            selection_started.set(false);
                            selected.set(0);
                            pending_caret.set(Some(0));
                        },
                        "Clear"
                    }
                }
            }
        }
    }
}
