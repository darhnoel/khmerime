use dioxus::prelude::*;

use crate::ui::editor::EditorSignals;

/// The ពាក្យផ្ទាល់ខ្លួន ("personal words") page: the session Ignore List — words the
/// user dismissed from the spell review, each un-ignorable. Session-only (cleared
/// on reload). See CONTEXT.md "Ignore List".
#[component]
pub(crate) fn PersonalWordsPage(state: EditorSignals, mut open: Signal<bool>) -> Element {
    let mut words = state.spell_ignore().into_iter().collect::<Vec<_>>();
    words.sort();

    rsx! {
        section { class: "saved-words-page", "data-testid": "personal-words-page",
            header { class: "saved-words-head",
                div { class: "saved-words-title",
                    button {
                        class: "saved-words-back",
                        "data-testid": "personal-words-back",
                        aria_label: "ត្រឡប់",
                        onclick: move |_| open.set(false),
                        "‹"
                    }
                    div {
                        h1 { "ពាក្យផ្ទាល់ខ្លួន" }
                        span { class: "saved-words-count", "{words.len()}" }
                    }
                }
            }

            div { class: "saved-words-content",
                if words.is_empty() {
                    div { class: "personal-words-empty",
                        div { class: "personal-words-empty-title", "មិនមានពាក្យផ្ទាល់ខ្លួននៅឡើយ" }
                        div { class: "personal-words-empty-hint",
                            "ចុច «មិនអើពើ» លើពាក្យដែលបានសម្គាល់ ដើម្បីបញ្ចូលវានៅទីនេះ។"
                        }
                    }
                } else {
                    ul { class: "personal-words-list",
                        for word in words.clone() {
                            li { class: "personal-words-row",
                                span { class: "personal-words-word", "{word}" }
                                button {
                                    class: "personal-words-remove",
                                    "data-testid": "unignore-word",
                                    title: "បញ្ចូលឡើងវិញ",
                                    aria_label: "បញ្ចូលឡើងវិញ",
                                    onclick: move |_| {
                                        let mut ignore = state.spell_ignore();
                                        ignore.remove(&word);
                                        state.spell_ignore.set(ignore);
                                    },
                                    "×"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
