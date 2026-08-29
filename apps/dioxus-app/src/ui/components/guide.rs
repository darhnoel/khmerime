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
fn ShortcutRow(keys: &'static str, label: &'static str) -> Element {
    rsx! {
        div { class: "guide-shortcut-row",
            kbd { "{keys}" }
            span { "{label}" }
        }
    }
}

#[component]
pub(crate) fn GuidePanel(show_guide: Signal<bool>) -> Element {
    rsx! {
        if show_guide() {
            aside {
                class: "guide",
                "data-testid": "guide-sheet",
                role: "dialog",
                aria_modal: "true",
                aria_labelledby: "guide-title",
                div { class: "guide-header",
                    button {
                        class: "guide-close",
                        "data-testid": "close-rules",
                        aria_label: "ត្រឡប់",
                        title: "Back",
                        onclick: move |_| show_guide.set(false),
                        "‹"
                    }
                    div {
                        h2 { id: "guide-title", "ក្បួន និងផ្លូវកាត់" }
                        p { "ជំនួយខ្លីៗសម្រាប់ការវាយអក្សរខ្មែរ" }
                    }
                }

                div { class: "guide-scroll",
                    section { class: "guide-section",
                        div { class: "guide-section-heading",
                            span { class: "guide-step", "01" }
                            div { h3 { "ការវាយធម្មតា" } p { "Normal typing" } }
                        }
                        div { class: "guide-shortcuts",
                            ShortcutRow { keys: "Space / ↑↓", label: "ប្ដូរជម្រើស" }
                            ShortcutRow { keys: "1–5", label: "ជ្រើសជម្រើសដែលមើលឃើញ" }
                            ShortcutRow { keys: "Enter", label: "បញ្ចូលពាក្យ ឬឃ្លា" }
                            ShortcutRow { keys: "Ctrl+Alt+K", label: "បើក ឬបិទការបម្លែង" }
                        }
                    }

                    section { class: "guide-section",
                        div { class: "guide-section-heading",
                            span { class: "guide-step", "02" }
                            div { h3 { "កែសម្រួលឃ្លា" } p { "Phrase & Segment Edit" } }
                        }
                        div { class: "guide-shortcuts",
                            ShortcutRow { keys: "Tab", label: "ចូល ឬចេញពីការកែពាក្យ" }
                            ShortcutRow { keys: "← →", label: "ផ្លាស់ទីរវាងពាក្យ" }
                            ShortcutRow { keys: "Space / ↑↓", label: "ប្ដូរជម្រើសនៃពាក្យ" }
                            ShortcutRow { keys: "Enter", label: "បញ្ចូលឃ្លាទាំងមូល" }
                        }
                    }

                    section { class: "guide-section",
                        div { class: "guide-section-heading",
                            span { class: "guide-step", "03" }
                            div { h3 { "សរសេរដោយដៃ" } p { "Manual character typing" } }
                        }
                        div { class: "guide-shortcuts",
                            ShortcutRow { keys: "Tab", label: "ប្ដូរប្រភេទតួអក្សរ" }
                            ShortcutRow { keys: "S", label: "រំលងអក្សររ៉ូម៉ាំង" }
                            ShortcutRow { keys: "U", label: "ថយក្រោយមួយជំហាន" }
                            ShortcutRow { keys: "Enter", label: "បញ្ចប់តួអក្សរ" }
                        }
                    }

                    section { class: "guide-section guide-rules",
                        div { class: "guide-section-heading",
                            span { class: "guide-step", "04" }
                            div { h3 { "ព្យញ្ជនៈពិសេស" } p { "Strict consonants" } }
                        }
                        div { class: "chip-grid",
                            for (khmer, roman) in STRICT_CONSONANTS {
                                div { class: "rule-chip",
                                    span { class: "chip-khmer", "{khmer}" }
                                    span { class: "chip-arrow", "→" }
                                    code { class: "chip-roman", "{roman}" }
                                }
                            }
                        }
                    }

                    section { class: "guide-section guide-rules",
                        div { class: "guide-section-heading",
                            span { class: "guide-step", "05" }
                            div { h3 { "ពាក្យពិសេស" } p { "Special words" } }
                        }
                        div { class: "example-list",
                            for (roman, khmer) in SPECIAL_WORDS {
                                div { class: "example-row",
                                    code { "{roman}" }
                                    span { class: "chip-arrow", "→" }
                                    strong { "{khmer}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
