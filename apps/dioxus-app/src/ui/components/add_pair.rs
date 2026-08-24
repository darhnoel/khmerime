use dioxus::prelude::*;

use crate::ui::editor::{replace_manual_save_request, EditorSignals, ManualSaveRequest, Preedit};

use super::FlickKeyboard;

#[component]
pub(crate) fn AddPairModal(
    state: EditorSignals,
    mut open: Signal<bool>,
    initial: Option<ManualSaveRequest>,
) -> Element {
    let initial_roman = initial.as_ref().map(|pair| pair.roman.clone()).unwrap_or_default();
    let initial_khmer = initial.as_ref().map(|pair| pair.khmer.clone()).unwrap_or_default();
    let mut roman = use_signal(|| initial_roman);
    let preedit = use_signal(|| Preedit::from_text(initial_khmer));
    let khmer = preedit().text();
    let can_save = !roman().trim().is_empty() && !khmer.is_empty();

    rsx! {
        button {
            class: "modal-scrim",
            aria_label: "បោះបង់",
            onclick: move |_| open.set(false),
        }
        section {
            class: "add-pair-modal",
            role: "dialog",
            aria_modal: "true",
            aria_labelledby: "add-pair-title",
            "data-testid": "add-pair-modal",

            div { class: "add-pair-head",
                div {
                    h2 { id: "add-pair-title", if initial.is_some() { "កែពាក្យ" } else { "បន្ថែមពាក្យ" } }
                    p { "រក្សាទុកអក្សរឡាតាំងជាមួយពាក្យខ្មែរដែលត្រឹមត្រូវ" }
                }
                button {
                    class: "settings-close",
                    aria_label: "បោះបង់",
                    onclick: move |_| open.set(false),
                    "✕"
                }
            }

            div { class: "add-pair-fields",
                label { class: "add-pair-field",
                    span { "អក្សរឡាតាំង" }
                    input {
                        "data-testid": "add-pair-roman",
                        value: "{roman}",
                        placeholder: "ឧ. khnhom",
                        autocomplete: "off",
                        spellcheck: "false",
                        autofocus: true,
                        oninput: move |event| roman.set(event.value()),
                    }
                }
                label { class: "add-pair-field",
                    span { "អក្សរខ្មែរ" }
                    input {
                        "data-testid": "add-pair-khmer",
                        value: "{khmer}",
                        placeholder: "បញ្ចូលដោយក្តារចុចខាងក្រោម",
                        readonly: true,
                    }
                }
            }

            FlickKeyboard { preedit }

            div { class: "add-pair-actions",
                button {
                    class: "ghost",
                    "data-testid": "add-pair-cancel",
                    onclick: move |_| open.set(false),
                    "បោះបង់"
                }
                button {
                    class: "mode-pill active",
                    "data-testid": "add-pair-save",
                    disabled: !can_save,
                    onclick: move |_| {
                        let request = ManualSaveRequest {
                            roman: roman(),
                            khmer: preedit().text(),
                        };
                        if replace_manual_save_request(initial.clone(), request, state) {
                            open.set(false);
                        }
                    },
                    "រក្សាទុក"
                }
            }
        }
    }
}
