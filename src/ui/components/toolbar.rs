use dioxus::prelude::*;

use crate::ui::components::segment_preview::SegmentPreview;
use crate::ui::storage::{save_enabled, save_font_size};
use crate::{MAX_FONT_SIZE, MIN_FONT_SIZE};

use crate::ui::editor::{
    dismiss_manual_save_request, save_manual_save_request, switch_input_mode, update_candidates, EditorSignals,
    InputMode,
};

#[component]
pub(crate) fn AppToolbar(state: EditorSignals, show_guide: Signal<bool>, font_size: Signal<usize>) -> Element {
    rsx! {
        div { class: "workspace-top",
            div { class: "toolbar",
                div { class: "font-tools",
                    label { class: "font-stepper-group",
                        span { class: "tool-label", "Font" }
                        input {
                            class: "font-stepper",
                            "data-testid": "font-size-input",
                            r#type: "number",
                            min: "{MIN_FONT_SIZE}",
                            max: "{MAX_FONT_SIZE}",
                            step: "2",
                            inputmode: "numeric",
                            aria_label: "Font size in pixels",
                            value: "{font_size()}",
                            oninput: move |event| {
                                let Ok(parsed_size) = event.value().parse::<usize>() else {
                                    return;
                                };
                                let next = parsed_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
                                font_size.set(next);
                                save_font_size(next, MIN_FONT_SIZE, MAX_FONT_SIZE);
                                if state.roman_enabled() {
                                    spawn(update_candidates(state.text(), state));
                                }
                            },
                        }
                        span { class: "font-stepper-unit", "px" }
                    }
                }
                div { class: "toolbar-segment-slot",
                    SegmentPreview { state }
                }
                div { class: "mode-tools",
                    if !state.engine_ready() {
                        div {
                            class: "engine-status loading",
                            "data-testid": "engine-status",
                            role: "status",
                            aria_label: "Preparing resources",
                            span { class: "engine-status-spinner", aria_hidden: "true" }
                        }
                    }
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
                    button {
                        class: if state.input_mode() == InputMode::NormalWordSuggestion { "mode-pill active" } else { "mode-pill" },
                        "data-testid": "mode-word",
                        onclick: move |_| switch_input_mode(InputMode::NormalWordSuggestion, state),
                        {InputMode::NormalWordSuggestion.label()}
                    }
                    button {
                        class: if state.input_mode() == InputMode::ManualCharacterTyping { "mode-pill active" } else { "mode-pill" },
                        "data-testid": "mode-manual",
                        onclick: move |_| switch_input_mode(InputMode::ManualCharacterTyping, state),
                        {InputMode::ManualCharacterTyping.label()}
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
                }
            }
            if let Some(request) = state.manual_save_request() {
                div { class: "manual-save-banner",
                    div { class: "manual-save-text",
                        span { class: "manual-save-label", "Manual compose" }
                        span { class: "manual-save-map", "{request.roman} → {request.khmer}" }
                    }
                    div { class: "manual-save-actions",
                        button {
                            class: "mode-pill active",
                            "data-testid": "manual-save",
                            onclick: move |_| {
                                let _ = save_manual_save_request(state);
                            },
                            "Save Mapping"
                        }
                        button {
                            class: "ghost",
                            "data-testid": "manual-save-dismiss",
                            onclick: move |_| dismiss_manual_save_request(state),
                            "Dismiss"
                        }
                    }
                }
            }
        }
    }
}
