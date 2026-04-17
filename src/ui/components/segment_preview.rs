use dioxus::prelude::*;

use crate::ui::editor::{move_segment_focus, normalized_suggestion_key, EditorSignals, InputMode};
use roman_lookup::DecoderMode;

fn segment_is_exact_recommended(segment_input: &str, selected_text: &str) -> bool {
    let selected_key = normalized_suggestion_key(selected_text);
    if selected_key.is_empty() {
        return false;
    }
    crate::engine(DecoderMode::Legacy)
        .exact_match_targets(segment_input)
        .into_iter()
        .any(|target| normalized_suggestion_key(&target) == selected_key)
}

#[component]
pub(crate) fn SegmentPreview(state: EditorSignals) -> Element {
    rsx! {
        div { class: "segment-scroll-wrapper",
            if state.input_mode() == InputMode::ManualCharacterTyping {
                if let Some(manual) = state.manual_typing_state() {
                    div { class: "segment-preview manual-preview", "data-testid": "manual-preview",
                        if !manual.composed_text.is_empty() {
                            div { class: "segment-chip active manual-chip",
                                span { class: "segment-chip-head",
                                    span { class: "segment-chip-output", "{manual.composed_text}" }
                                }
                                span { class: "segment-chip-input", "built" }
                            }
                        }
                        div { class: "segment-chip manual-chip",
                            span { class: "segment-chip-head",
                                span { class: "segment-chip-output", "{manual.remaining_roman()}" }
                            }
                            span { class: "segment-chip-input", "next {manual.expected_kind.label()}" }
                        }
                    }
                }
            } else if let Some(session) = state.segmented_session() {
                div { class: "segment-preview", "data-testid": "segment-preview",
                    for (index, segment) in session.segments.iter().enumerate() {
                        button {
                            key: "{index}-{segment.input}",
                            class: if state.segmented_refine_mode() && index == session.focused { "segment-chip active" } else { "segment-chip" },
                            title: "{segment.selected_text()} ({segment.input})",
                            onclick: move |_| {
                                if move_segment_focus(index as isize - session.focused as isize, state) {
                                    state.number_pick_mode.set(false);
                                }
                            },
                            span { class: "segment-chip-head",
                                span { class: "segment-chip-output", "{segment.selected_text()}" }
                                if segment_is_exact_recommended(&segment.input, &segment.selected_text()) {
                                    span { class: "segment-chip-recommended", "គួរប្រើ" }
                                }
                            }
                            span { class: "segment-chip-input", "{segment.input}" }
                        }
                    }
                }
            } else if state.roman_enabled() {
                div { class: "segment-preview segment-preview-skeleton", aria_hidden: "true",
                    span { class: "segment-placeholder-chip segment-placeholder-chip-1" }
                    span { class: "segment-placeholder-chip segment-placeholder-chip-3" }
                    span { class: "segment-placeholder-chip segment-placeholder-chip-2" }
                }
            }
        }
    }
}
