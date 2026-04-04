use std::collections::HashMap;

use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation};

use crate::ui::editor::{update_candidates, SegmentedSession};
use crate::ui::storage::{save_editor_text, save_enabled, save_font_size};
use crate::{CompositionMark, SuggestionPopup, MAX_FONT_SIZE, MIN_FONT_SIZE};

#[component]
pub(crate) fn AppToolbar(
    engine_ready: Signal<bool>,
    text: Signal<String>,
    roman_enabled: Signal<bool>,
    decoder_mode: Signal<DecoderMode>,
    show_guide: Signal<bool>,
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
                            if roman_enabled() {
                                spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), engine_ready, suggestions, popup, composition, shadow_debug, segmented_session, segmented_refine_mode, active_token, number_pick_mode, selection_started, selected, history));
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
                                spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), engine_ready, suggestions, popup, composition, shadow_debug, segmented_session, segmented_refine_mode, active_token, number_pick_mode, selection_started, selected, history));
                            }
                        },
                        "A+"
                    }
                }
                div { class: "mode-tools",
                    button {
                        class: if roman_enabled() { "mode-pill active" } else { "mode-pill" },
                        "data-testid": "toggle-live-edit",
                        onclick: move |_| {
                            let next = !roman_enabled();
                            roman_enabled.set(next);
                            save_enabled(next);
                            if next {
                                spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), engine_ready, suggestions, popup, composition, shadow_debug, segmented_session, segmented_refine_mode, active_token, number_pick_mode, selection_started, selected, history));
                            } else {
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
                            }
                        },
                        if roman_enabled() {
                            "Live Edit"
                        } else {
                            "Live Edit Off"
                        }
                    }
                    if !engine_ready() {
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
                            text.set(String::new());
                            save_editor_text("");
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
                            pending_caret.set(Some(0));
                        },
                        "Clear"
                    }
                }
            }
        }
    }
}
