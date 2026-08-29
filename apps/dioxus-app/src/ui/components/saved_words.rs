use dioxus::prelude::*;

use crate::ui::editor::{remove_user_dictionary_mapping, save_manual_save_request, EditorSignals, ManualSaveRequest};

use super::AddPairModal;

const PAGE_SIZE: usize = 20;

#[cfg(target_arch = "wasm32")]
async fn undo_timeout() {
    gloo_timers::future::TimeoutFuture::new(5_000).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn undo_timeout() {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}

#[component]
pub(crate) fn SavedWordsPage(state: EditorSignals, mut open: Signal<bool>) -> Element {
    let mut query = use_signal(String::new);
    let mut page = use_signal(|| 0usize);
    let mut show_add_pair = use_signal(|| false);
    let mut editing = use_signal(|| None::<ManualSaveRequest>);
    let mut open_menu = use_signal(|| None::<String>);
    let mut undo_pair = use_signal(|| None::<ManualSaveRequest>);

    let mut entries = state
        .user_dictionary()
        .into_iter()
        .flat_map(|(roman, values)| {
            values.into_iter().map(move |khmer| ManualSaveRequest {
                roman: roman.clone(),
                khmer,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.roman.cmp(&right.roman).then_with(|| left.khmer.cmp(&right.khmer)));

    let normalized_query = query().trim().to_lowercase();
    let filtered = entries
        .iter()
        .filter(|pair| {
            normalized_query.is_empty()
                || pair.roman.to_lowercase().contains(&normalized_query)
                || pair.khmer.contains(query().trim())
        })
        .cloned()
        .collect::<Vec<_>>();
    let page_count = filtered.len().div_ceil(PAGE_SIZE).max(1);
    let current_page = page().min(page_count - 1);
    let page_entries = filtered
        .iter()
        .skip(current_page * PAGE_SIZE)
        .take(PAGE_SIZE)
        .cloned()
        .collect::<Vec<_>>();

    rsx! {
        section { class: "saved-words-page", "data-testid": "saved-words-page",
            header { class: "saved-words-head",
                div { class: "saved-words-title",
                    button {
                        class: "saved-words-back",
                        "data-testid": "saved-words-back",
                        aria_label: "ត្រឡប់",
                        onclick: move |_| open.set(false),
                        "‹"
                    }
                    div {
                        h1 { "ពាក្យរក្សាទុក" }
                        span { class: "saved-words-count", "{entries.len()}" }
                    }
                }
                button {
                    class: "saved-words-add",
                    "data-testid": "add-saved-mapping",
                    aria_label: "បន្ថែមពាក្យ",
                    onclick: move |_| {
                        editing.set(None);
                        show_add_pair.set(true);
                    },
                    "+"
                }
            }

            div { class: "saved-words-content",
                label { class: "saved-words-search",
                    span { class: "sr-only", "ស្វែងរកពាក្យ" }
                    input {
                        "data-testid": "saved-words-search",
                        value: "{query}",
                        placeholder: "ស្វែងរកអក្សរឡាតាំង ឬខ្មែរ…",
                        oninput: move |event| {
                            query.set(event.value());
                            page.set(0);
                        }
                    }
                }

                div { class: "saved-words-table-wrap",
                    table { class: "saved-words-table",
                        thead { tr {
                            th { "អក្សរឡាតាំង" }
                            th { "អក្សរខ្មែរ" }
                            th { class: "saved-words-actions-head", "សកម្មភាព" }
                        } }
                        tbody {
                            if page_entries.is_empty() {
                                tr { td { colspan: "3", class: "saved-words-empty",
                                    if normalized_query.is_empty() { "មិនទាន់មានពាក្យរក្សាទុកទេ។" }
                                    else { "រកមិនឃើញពាក្យទេ។" }
                                } }
                            } else {
                                for pair in page_entries.iter() {
                                    {
                                        let row_key = format!("{}\u{0}{}", pair.roman, pair.khmer);
                                        rsx! { tr { class: "saved-words-row", key: "{row_key}",
                                            td { class: "saved-word-roman", "{pair.roman}" }
                                            td { class: "saved-word-khmer", "{pair.khmer}" }
                                            td { class: "saved-word-actions",
                                                button {
                                                    class: "saved-word-menu-button",
                                                    "data-testid": "saved-word-menu-button",
                                                    aria_label: "សកម្មភាពសម្រាប់ {pair.roman}",
                                                    onclick: {
                                                        let row_key = row_key.clone();
                                                        move |_| {
                                                            open_menu.set(if open_menu().as_deref() == Some(row_key.as_str()) {
                                                                None
                                                            } else {
                                                                Some(row_key.clone())
                                                            });
                                                        }
                                                    },
                                                    "⋯"
                                                }
                                                if open_menu().as_deref() == Some(row_key.as_str()) {
                                                    div { class: "saved-word-menu",
                                                        button { onclick: {
                                                            let pair = pair.clone();
                                                            move |_| {
                                                                editing.set(Some(pair.clone()));
                                                                open_menu.set(None);
                                                                show_add_pair.set(true);
                                                            }
                                                        }, "data-testid": "edit-saved-word", "កែប្រែ" }
                                                        button { class: "danger", "data-testid": "delete-saved-word", onclick: {
                                                            let pair = pair.clone();
                                                            move |_| {
                                                                if remove_user_dictionary_mapping(&pair.roman, &pair.khmer, state) {
                                                                    undo_pair.set(Some(pair.clone()));
                                                                    let deleted = pair.clone();
                                                                    spawn(async move {
                                                                        undo_timeout().await;
                                                                        if undo_pair() == Some(deleted) {
                                                                            undo_pair.set(None);
                                                                        }
                                                                    });
                                                                }
                                                                open_menu.set(None);
                                                            }
                                                        }, "លុប" }
                                                    }
                                                }
                                            }
                                        } }
                                    }
                                }
                            }
                        }
                    }
                }

                if page_count > 1 {
                    nav { class: "saved-words-pagination", aria_label: "ទំព័រពាក្យរក្សាទុក",
                        button {
                            disabled: current_page == 0,
                            onclick: move |_| page.set(current_page.saturating_sub(1)),
                            "‹ មុន"
                        }
                        span { "{current_page + 1} / {page_count}" }
                        button {
                            disabled: current_page + 1 >= page_count,
                            onclick: move |_| page.set((current_page + 1).min(page_count - 1)),
                            "បន្ទាប់ ›"
                        }
                    }
                }
            }
        }

        if let Some(pair) = undo_pair() {
            div { class: "saved-words-toast", "data-testid": "saved-words-toast", role: "status",
                span { "បានលុប {pair.roman} → {pair.khmer}" }
                button { "data-testid": "undo-delete-saved-word", onclick: move |_| {
                    let _ = save_manual_save_request(pair.clone(), state);
                    undo_pair.set(None);
                }, "មិនធ្វើវិញ" }
            }
        }

        if show_add_pair() {
            AddPairModal { state, open: show_add_pair, initial: editing() }
        }
    }
}
