use dioxus::prelude::*;

use crate::ui::components::SavedWordsPage;
use crate::ui::editor::{update_candidates, EditorSignals};
use crate::ui::spellcheck::{
    check_text, check_via_api, mark_detector_unavailable,
    wait_for_clear_confirmation, yield_before_check, SpellReview, SpellReviewStatus,
};
use crate::ui::storage::{save_enabled, save_font_size, save_sidebar_open, Palette, Theme};
use crate::{engine, EngineReadiness, MAX_FONT_SIZE, MIN_FONT_SIZE};

#[component]
fn Icon(name: &'static str) -> Element {
    let path = match name {
        "menu" => rsx! { path { d: "M4 7h16M4 12h16M4 17h16" } },
        "pen" => rsx! {
            path { d: "M12 20h9" }
            path { d: "M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z" }
        },
        "list" => rsx! {
            path { d: "M8 6h13M8 12h13M8 18h13" }
            path { d: "M3 6h.01M3 12h.01M3 18h.01" }
        },
        "keyboard" => rsx! {
            rect { x: "2", y: "6", width: "20", height: "12", rx: "2" }
            path { d: "M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10" }
        },
        "book" => rsx! {
            path { d: "M4 19.5A2.5 2.5 0 0 1 6.5 17H20" }
            path { d: "M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2Z" }
        },
        "bookmark" => rsx! { path { d: "M6 3h12v18l-6-4-6 4Z" } },
        "check" => rsx! {
            circle { cx: "12", cy: "12", r: "9" }
            path { d: "m8 12 3 3 5-6" }
        },
        "gear" => rsx! {
            circle { cx: "12", cy: "12", r: "3" }
            path { d: "M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06-.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09A1.65 1.65 0 0 0 15 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09A1.65 1.65 0 0 0 19.4 15Z" }
        },
        "collapse" => rsx! { path { d: "m15 18-6-6 6-6" } },
        _ => rsx! {},
    };
    rsx! {
        svg {
            width: "18", height: "18", view_box: "0 0 24 24", fill: "none",
            stroke: "currentColor", stroke_width: "1.7", stroke_linecap: "round",
            stroke_linejoin: "round", {path}
        }
    }
}

#[component]
pub(crate) fn AppToolbar(
    state: EditorSignals,
    show_guide: Signal<bool>,
    font_size: Signal<usize>,
    theme: Signal<Theme>,
    palette: Signal<Palette>,
    sidebar_open: Signal<bool>,
) -> Element {
    let mut show_saved_dictionary = use_signal(|| false);
    let mut show_settings = use_signal(|| false);
    let mut saved_entries = state
        .user_dictionary()
        .into_iter()
        .flat_map(|(roman, values)| values.into_iter().map(move |khmer| (roman.clone(), khmer)))
        .collect::<Vec<_>>();
    saved_entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let spell_review = state.spell_review();
    let spell_checking = spell_review.status == SpellReviewStatus::Checking;
    let spell_issue_count = spell_review.issues.len();

    rsx! {
        header { class: "topbar",
            div { class: "topbar-brand",
                button {
                    class: "chrome-button sidebar-toggle", "data-testid": "toggle-sidebar",
                    title: "Toggle sidebar", aria_label: "បើក ឬបិទរបារចំហៀង",
                    aria_expanded: "{sidebar_open()}",
                    onclick: move |_| {
                        let next = !sidebar_open();
                        sidebar_open.set(next);
                        save_sidebar_open(next);
                    },
                    Icon { name: "menu" }
                }
                span { class: "wordmark", "KhmerIME" }
                span { class: "beta-badge", "Beta" }
            }
            div { class: "topbar-right",
                if state.engine_readiness() == EngineReadiness::Booting {
                    div { class: "engine-status loading", "data-testid": "engine-status", role: "status",
                        span { class: "engine-status-spinner", aria_hidden: "true" }
                        span { "កំពុងរៀបចំ…" }
                    }
                } else if state.engine_readiness() == EngineReadiness::LegacyReady {
                    div { class: "engine-status partial", "data-testid": "engine-status", role: "status", "កំពុងបំពេញទិន្នន័យ…" }
                } else if state.engine_readiness() == EngineReadiness::Failed {
                    div { class: "engine-status error", "data-testid": "engine-status", role: "status", "ម៉ាស៊ីនមានបញ្ហា" }
                }
                button {
                    class: if show_settings() { "chrome-button active" } else { "chrome-button" },
                    "data-testid": "toggle-settings", title: "Settings", aria_label: "ការកំណត់",
                    onclick: move |_| show_settings.set(!show_settings()), Icon { name: "gear" }
                }
            }
        }

        if sidebar_open() {
            button { class: "sidebar-scrim", aria_label: "បិទរបារចំហៀង",
                onclick: move |_| { sidebar_open.set(false); save_sidebar_open(false); } }
        }

        aside { class: "app-sidebar", aria_label: "ការរុករក និងឧបករណ៍",
            nav { class: "sidebar-nav",
                section { class: "sidebar-section sidebar-behavior",
                    h2 { "ការបម្លែង" }
                    button {
                        class: "sidebar-item sidebar-toggle-item", "data-testid": "toggle-live-edit",
                        aria_pressed: "{state.roman_enabled()}",
                        onclick: move |_| {
                            let next = !state.roman_enabled();
                            state.roman_enabled.set(next);
                            save_enabled(next);
                            if next { spawn(update_candidates(state.text(), state)); }
                            else { state.clear_candidate_state_and_picker(); }
                        },
                        Icon { name: "pen" } span { "ប្រើ KhmerIME" }
                        span { class: if state.roman_enabled() { "toggle-switch on" } else { "toggle-switch" }, aria_hidden: "true" }
                    }
                }
                section { class: "sidebar-section",
                    h2 { "ឧបករណ៍" }
                    button {
                        class: if spell_review.result_bar_visible() { "sidebar-item active" } else { "sidebar-item" },
                        "data-testid": "check-spelling",
                        aria_pressed: "{spell_review.result_bar_visible()}",
                        disabled: spell_checking,
                        onclick: move |_| {
                            // Toggle: if a review is already shown, clicking clears it.
                            if spell_review.result_bar_visible() {
                                state.clear_spell_review();
                                return;
                            }
                            let snapshot = state.text();
                            let saved = state.user_dictionary();
                            state.clear_candidate_state_and_picker();
                            state.spell_review.set(SpellReview::checking());
                            spawn(async move {
                                yield_before_check().await;
                                let entries = engine(roman_lookup::DecoderMode::Legacy).entries();
                                // Primary: our 8901 API (0.9857 segmenter + decomposition + RAC).
                                // Fallback: the local Rust dictionary check if the API is down.
                                let result = match check_via_api(&snapshot).await {
                                    Ok(api_result) => api_result,
                                    Err(_error) => {
                                        #[cfg(target_arch = "wasm32")]
                                        web_sys::console::warn_1(
                                            &format!("spellcheck API unavailable; local dictionary fallback: {_error}").into(),
                                        );
                                        mark_detector_unavailable(check_text(&snapshot, entries, &saved))
                                    }
                                };
                                if state.text() == snapshot {
                                    let is_clear = result.issues.is_empty();
                                    state.spell_review.set(SpellReview::complete(result));
                                    if is_clear {
                                        wait_for_clear_confirmation().await;
                                        let review = state.spell_review();
                                        if state.text() == snapshot
                                            && review.status == SpellReviewStatus::Complete
                                            && review.issues.is_empty()
                                        {
                                            state.clear_spell_review();
                                        }
                                    }
                                }
                            });
                        },
                        Icon { name: "check" }
                        span {
                            if spell_checking { "កំពុងពិនិត្យ…" } else { "ពិនិត្យអក្ខរាវិរុទ្ធ" }
                        }
                        if spell_issue_count > 0 {
                            span { class: "sidebar-count", "{spell_issue_count}" }
                        }
                    }
                    button {
                        class: if show_guide() { "sidebar-item active" } else { "sidebar-item" },
                        "data-testid": "toggle-rules", aria_pressed: "{show_guide()}",
                        onclick: move |_| show_guide.set(!show_guide()),
                        Icon { name: "book" } span { "ក្បួន និងផ្លូវកាត់" }
                    }
                    button {
                        class: if show_saved_dictionary() { "sidebar-item active" } else { "sidebar-item" },
                        "data-testid": "toggle-saved-mappings", aria_pressed: "{show_saved_dictionary()}",
                        onclick: move |_| show_saved_dictionary.set(true),
                        Icon { name: "bookmark" } span { "ពាក្យរក្សាទុក" }
                        span { class: "sidebar-count", "{saved_entries.len()}" }
                        span { class: "sidebar-chevron", aria_hidden: "true", "›" }
                    }
                }
            }
            button { class: "sidebar-collapse", aria_label: "បង្រួមរបារចំហៀង",
                onclick: move |_| { sidebar_open.set(false); save_sidebar_open(false); },
                Icon { name: "collapse" } span { "បង្រួមរបារ" }
            }
        }

        if show_saved_dictionary() {
            SavedWordsPage { state, open: show_saved_dictionary }
        }

        if show_settings() {
            button { class: "modal-scrim", aria_label: "បិទការកំណត់", onclick: move |_| show_settings.set(false) }
            section { class: "settings-modal", role: "dialog", aria_modal: "true", aria_labelledby: "settings-title",
                div { class: "settings-head",
                    div { h2 { id: "settings-title", "ការកំណត់" } p { "កែរូបរាង និងបទពិសោធន៍សរសេរ" } }
                    button { class: "settings-close", "data-testid": "settings-close", aria_label: "បិទការកំណត់",
                        onclick: move |_| show_settings.set(false), "✕" }
                }
                div { class: "settings-group",
                    h3 { "រូបរាង" }
                    div { class: "settings-row",
                        span { class: "settings-label", "ពន្លឺ" }
                        div { class: "theme-switch", role: "group", aria_label: "ពន្លឺ",
                            button { "data-testid": "theme-light", class: if theme() == Theme::Light { "active" } else { "" }, onclick: move |_| theme.set(Theme::Light), "ភ្លឺ" }
                            button { "data-testid": "theme-dark", class: if theme() == Theme::Dark { "active" } else { "" }, onclick: move |_| theme.set(Theme::Dark), "ងងឹត" }
                        }
                    }
                    div { class: "settings-row settings-palette-row",
                        span { class: "settings-label", "ពណ៌" }
                        div { class: "palette-switch", role: "group", aria_label: "ពណ៌",
                            button { "data-testid": "palette-default", class: if palette() == Palette::Default { "active" } else { "" }, onclick: move |_| palette.set(Palette::Default),
                                span { class: "palette-dot default", aria_hidden: "true" } span { "ដើម" }
                            }
                            button { "data-testid": "palette-angkor", class: if palette() == Palette::Angkor { "active" } else { "" }, onclick: move |_| palette.set(Palette::Angkor),
                                span { class: "palette-dot angkor", aria_hidden: "true" } span { "អង្គរ" }
                            }
                            button { "data-testid": "palette-lotus", class: if palette() == Palette::Lotus { "active" } else { "" }, onclick: move |_| palette.set(Palette::Lotus),
                                span { class: "palette-dot lotus", aria_hidden: "true" } span { "ផ្កាឈូក" }
                            }
                            button { "data-testid": "palette-forest", class: if palette() == Palette::Forest { "active" } else { "" }, onclick: move |_| palette.set(Palette::Forest),
                                span { class: "palette-dot forest", aria_hidden: "true" } span { "ព្រៃឈើ" }
                            }
                        }
                    }
                    div { class: "settings-row",
                        span { class: "settings-label", "ទំហំអក្សរ" }
                        label { class: "font-stepper-group",
                            input {
                                class: "font-stepper", "data-testid": "font-size-input", r#type: "number",
                                min: "{MIN_FONT_SIZE}", max: "{MAX_FONT_SIZE}", step: "2", inputmode: "numeric",
                                aria_label: "ទំហំអក្សរជាភីកសែល", value: "{font_size()}",
                                oninput: move |event| {
                                    let Ok(parsed_size) = event.value().parse::<usize>() else { return; };
                                    let next = parsed_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
                                    font_size.set(next);
                                    save_font_size(next, MIN_FONT_SIZE, MAX_FONT_SIZE);
                                    if state.roman_enabled() { spawn(update_candidates(state.text(), state)); }
                                }
                            }
                            span { class: "font-stepper-unit", "px" }
                        }
                    }
                }
            }
        }
    }
}
