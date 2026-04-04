use dioxus::prelude::*;

const STRICT_CONSONANTS: [(&str, &str); 8] = [
    ("គ", "g"),
    ("ឃ", "gh"),
    ("ជ", "j"),
    ("ឈ", "jh"),
    ("ទ", "tt"),
    ("ធ", "tth"),
    ("ផ", "bh"),
    ("អ", "or"),
];

const SPECIAL_WORDS: [(&str, &str); 5] = [
    ("laor", "ល្អ"),
    ("saork", "ស្អក"),
    ("chhaov", "ឆ្អៅ"),
    ("bhaav", "ផ្អាវ"),
    ("bhaor", "ផ្អ"),
];

#[component]
pub(crate) fn GuidePanel(show_guide: Signal<bool>) -> Element {
    rsx! {
        if show_guide() {
            aside { class: "guide guide-open",
                button {
                    class: "guide-handle guide-handle-open",
                    onclick: move |_| show_guide.set(false),
                    title: "Hide typing rules",
                    "<"
                }
                div { class: "guide-panel",
                    div { class: "guide-card intro",
                        h2 { "Typing rules" }
                        p { "These are the patterns users should remember first. The goal is fast recall, not full linguistic detail." }
                    }
                    div { class: "guide-card",
                        h3 { "Strict consonants" }
                        div { class: "chip-grid",
                            for (khmer, roman) in STRICT_CONSONANTS {
                                div { class: "rule-chip",
                                    span { class: "chip-khmer", "{khmer}" }
                                    span { class: "chip-arrow", "→" }
                                    span { class: "chip-roman", "{roman}" }
                                }
                            }
                        }
                    }
                    div { class: "guide-card",
                        h3 { "Special words" }
                        div { class: "example-list",
                            for (roman, khmer) in SPECIAL_WORDS {
                                div { class: "example-row",
                                    code { "{roman}" }
                                    span { "{khmer}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
