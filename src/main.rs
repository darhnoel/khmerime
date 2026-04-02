use std::collections::HashMap;
use std::sync::OnceLock;

use dioxus::document;
use dioxus::html::Modifiers;
use dioxus::prelude::*;
use roman_lookup::{DecoderConfig, DecoderMode, ShadowObservation, Transliterator};

#[cfg(target_arch = "wasm32")]
use web_sys::wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::window;

static STYLES: Asset = asset!("/assets/main.css");
static LEGACY_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
static SHADOW_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();

const EDITOR_ID: &str = "ime-editor";
#[cfg(target_arch = "wasm32")]
const STORAGE_TEXT: &str = "roman_lookup.text";
#[cfg(target_arch = "wasm32")]
const STORAGE_ENABLED: &str = "roman_lookup.enabled";
#[cfg(target_arch = "wasm32")]
const STORAGE_DECODER_MODE: &str = "roman_lookup.decoder_mode";
#[cfg(target_arch = "wasm32")]
const STORAGE_HISTORY: &str = "roman_lookup.history";
#[cfg(target_arch = "wasm32")]
const STORAGE_FONT_SIZE: &str = "roman_lookup.font_size";
const DEFAULT_FONT_SIZE: usize = 24;
const MIN_FONT_SIZE: usize = 18;
const MAX_FONT_SIZE: usize = 38;
const VISIBLE_SUGGESTIONS: usize = 5;
const FALLBACK_POPUP_LEFT: f64 = 18.0;
const FALLBACK_POPUP_TOP: f64 = 88.0;

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

#[derive(Clone, Copy, Debug, PartialEq)]
struct SuggestionPopup {
    left: f64,
    top: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct CompositionMark {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    dioxus::launch(App);
}

fn engine(mode: DecoderMode) -> &'static Transliterator {
    match mode {
        DecoderMode::Legacy => LEGACY_TRANSLITERATOR
            .get_or_init(|| Transliterator::from_default_data().expect("embedded lexicon must load")),
        DecoderMode::Shadow | DecoderMode::Wfst | DecoderMode::Hybrid => SHADOW_TRANSLITERATOR.get_or_init(|| {
            Transliterator::from_default_data_with_config(
                DecoderConfig::default()
                    .with_mode(DecoderMode::Shadow)
                    .with_shadow_log(false),
            )
            .expect("embedded lexicon must load")
        }),
    }
}

#[component]
fn App() -> Element {
    let mut text = use_signal(load_editor_text);
    let mut roman_enabled = use_signal(load_enabled);
    let mut decoder_mode = use_signal(load_decoder_mode);
    let mut font_size = use_signal(load_font_size);
    let mut show_guide = use_signal(|| false);
    let mut suggestions = use_signal(Vec::<String>::new);
    let mut popup = use_signal(|| None::<SuggestionPopup>);
    let mut composition = use_signal(|| None::<CompositionMark>);
    let mut shadow_debug = use_signal(|| None::<ShadowObservation>);
    let mut ui_status = use_signal(String::new);
    let mut active_token = use_signal(String::new);
    let mut number_pick_mode = use_signal(|| false);
    let mut selection_started = use_signal(|| false);
    let mut selected = use_signal(|| 0usize);
    let mut pending_caret = use_signal(|| None::<usize>);
    let history = use_signal(load_history);

    use_effect(move || {
        if let Some(caret) = pending_caret() {
            set_editor_caret(caret);
            pending_caret.set(None);
        }
    });

    use_effect(move || {
        let _ = engine(DecoderMode::Legacy);
        let _ = engine(decoder_mode());
    });

    use_effect(move || {
        if suggestions().is_empty() {
            popup.set(None);
            number_pick_mode.set(false);
            selection_started.set(false);
            return;
        }
        spawn(refresh_popup_position(popup));
    });

    rsx! {
        document::Stylesheet { href: STYLES }
        div { class: "shell",
            div { class: if show_guide() { "board" } else { "board board-wide" },
                section { class: "workspace",
                    div { class: "workspace-top",
                        div { class: "hero",
                            h1 { "Open Khmer" }
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
                                        save_font_size(next);
                                        ui_status.set(format!("font={}px", next));
                                        if roman_enabled() {
                                            spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, ui_status, active_token, number_pick_mode, selection_started, selected, history));
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
                                        save_font_size(next);
                                        ui_status.set(format!("font={}px", next));
                                        if roman_enabled() {
                                            spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, ui_status, active_token, number_pick_mode, selection_started, selected, history));
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
                                                ui_status.set("mode=roman".to_owned());
                                                spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, ui_status, active_token, number_pick_mode, selection_started, selected, history));
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
                                                ui_status.set("mode=raw".to_owned());
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
                                                    ui_status.set("decoder=legacy".to_owned());
                                                    if roman_enabled() {
                                                        spawn(update_candidates(text(), text, roman_enabled, DecoderMode::Legacy, suggestions, popup, composition, shadow_debug, ui_status, active_token, number_pick_mode, selection_started, selected, history));
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
                                                    ui_status.set("decoder=shadow".to_owned());
                                                    if roman_enabled() {
                                                        spawn(update_candidates(text(), text, roman_enabled, DecoderMode::Shadow, suggestions, popup, composition, shadow_debug, ui_status, active_token, number_pick_mode, selection_started, selected, history));
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
                                        ui_status.set("cleared".to_owned());
                                    },
                                    "Clear"
                                }
                            }
                        }
                    }
                    div {
                        class: if roman_enabled() && decoder_mode() == DecoderMode::Shadow {
                            "workspace-body workspace-body-shadow"
                        } else {
                            "workspace-body"
                        },
                        if roman_enabled() && decoder_mode() == DecoderMode::Shadow {
                            div { class: "guide-card debug-card shadow-side-panel",
                                "data-testid": "shadow-panel",
                                div { class: "card-head",
                                    div {
                                        h2 { "Shadow Compare" }
                                        p { "Legacy suggestions remain visible. This panel shows the shadow WFST comparison for the active token." }
                                    }
                                }
                                if let Some(debug) = shadow_debug() {
                                    div { class: "debug-grid",
                                        div { class: "debug-row",
                                            span { class: "debug-label", "Input" }
                                            code { class: "debug-value", {debug.input.clone()} }
                                        }
                                        div { class: "debug-row",
                                            span { class: "debug-label", "Mismatch" }
                                            code { class: "debug-value", {debug.mismatch.as_str()} }
                                        }
                                        div { class: "debug-row",
                                            span { class: "debug-label", "Legacy Top" }
                                            span { class: "debug-value", {debug.legacy_top.clone().unwrap_or_else(|| "-".to_owned())} }
                                        }
                                        div { class: "debug-row",
                                            span { class: "debug-label", "WFST Top" }
                                            span { class: "debug-value", {debug.wfst_top.clone().unwrap_or_else(|| "-".to_owned())} }
                                        }
                                        div { class: "debug-row",
                                            span { class: "debug-label", "WFST Failure" }
                                            code { class: "debug-value", {debug.wfst_failure.clone().unwrap_or_else(|| "-".to_owned())} }
                                        }
                                        div { class: "debug-row",
                                            span { class: "debug-label", "Legacy Top-5" }
                                            span { class: "debug-value", {debug.legacy_top5.join(" | ")} }
                                        }
                                        div { class: "debug-row",
                                            span { class: "debug-label", "WFST Top-5" }
                                            span { class: "debug-value", {debug.wfst_top5.join(" | ")} }
                                        }
                                        div { class: "debug-row",
                                            span { class: "debug-label", "Latency" }
                                            code { class: "debug-value", {format!(
                                                "legacy {}us / wfst {}us",
                                                debug.legacy_latency_us,
                                                debug.wfst_latency_us
                                                    .map(|value| value.to_string())
                                                    .unwrap_or_else(|| "-".to_owned())
                                            )} }
                                        }
                                    }
                                } else {
                                    div { class: "empty-state",
                                        p { "No active token" }
                                        span { "Type a roman token to inspect the shadow comparison." }
                                    }
                                }
                            }
                        }
                        div { class: "editor-card",
                            div { class: "editor-wrap",
                                textarea {
                                    id: EDITOR_ID,
                                    "data-testid": "editor-input",
                                    class: "editor",
                                    style: "font-size: {font_size()}px;",
                                    value: "{text}",
                                    placeholder: "Type roman text here...",
                                    spellcheck: "false",
                                    autocomplete: "off",
                                    autocorrect: "off",
                                    oninput: move |event| {
                                        let value = event.value();
                                        save_editor_text(&value);
                                        text.set(value.clone());
                                        ui_status.set(format!("input len={} decoder={:?}", value.chars().count(), decoder_mode()));
                                        spawn(update_candidates(value, text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, ui_status, active_token, number_pick_mode, selection_started, selected, history));
                                    },
                                    onkeydown: move |event| {
                                        let key = event.key().to_string();
                                        let modifiers = event.modifiers();

                                        if modifiers.contains(Modifiers::ALT)
                                            && modifiers.contains(Modifiers::CONTROL)
                                            && key.eq_ignore_ascii_case("k")
                                        {
                                            event.prevent_default();
                                            let next = !roman_enabled();
                                            roman_enabled.set(next);
                                            save_enabled(next);
                                            if !next {
                                                suggestions.set(Vec::new());
                                                popup.set(None);
                                                composition.set(None);
                                                shadow_debug.set(None);
                                                active_token.set(String::new());
                                                number_pick_mode.set(false);
                                                selection_started.set(false);
                                                selected.set(0);
                                            } else {
                                                spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, ui_status, active_token, number_pick_mode, selection_started, selected, history));
                                            }
                                            return;
                                        }

                                        if !roman_enabled() {
                                            return;
                                        }

                                        match key.as_str() {
                                            "Tab" if !suggestions().is_empty() => {
                                                event.prevent_default();
                                                let len = suggestions().len();
                                                selected.set((selected() + 1) % len);
                                                number_pick_mode.set(true);
                                                selection_started.set(true);
                                            }
                                            "ArrowDown" if !suggestions().is_empty() => {
                                                event.prevent_default();
                                                let len = suggestions().len();
                                                if !selection_started() {
                                                    selected.set(0);
                                                } else {
                                                    selected.set((selected() + 1) % len);
                                                }
                                                number_pick_mode.set(true);
                                                selection_started.set(true);
                                            }
                                            "ArrowUp" if !suggestions().is_empty() => {
                                                event.prevent_default();
                                                let len = suggestions().len();
                                                if !selection_started() {
                                                    selected.set(len.saturating_sub(1));
                                                } else {
                                                    selected.set((selected() + len - 1) % len);
                                                }
                                                number_pick_mode.set(true);
                                                selection_started.set(true);
                                            }
                                            key if is_space_key(key) && modifiers.contains(Modifiers::SHIFT) && !suggestions().is_empty() => {
                                                event.prevent_default();
                                                spawn(commit_selection(
                                                    false,
                                                    text,
                                                    suggestions,
                                                    popup,
                                                    composition,
                                                    active_token,
                                                    selection_started,
                                                    selected,
                                                    pending_caret,
                                                    history,
                                                ));
                                            }
                                            key if is_space_key(key) && !suggestions().is_empty() && !selection_started() => {
                                                event.prevent_default();
                                                selected.set(0);
                                                number_pick_mode.set(true);
                                                selection_started.set(true);
                                            }
                                            key if is_space_key(key) && !suggestions().is_empty() => {
                                                event.prevent_default();
                                                let len = suggestions().len();
                                                selected.set((selected() + 1) % len);
                                                number_pick_mode.set(true);
                                                selection_started.set(true);
                                            }
                                            "Enter" if !suggestions().is_empty() => {
                                                event.prevent_default();
                                                spawn(commit_selection(
                                                    false,
                                                    text,
                                                    suggestions,
                                                    popup,
                                                    composition,
                                                    active_token,
                                                    selection_started,
                                                    selected,
                                                    pending_caret,
                                                    history,
                                                ));
                                            }
                                            key if number_pick_mode() && !suggestions().is_empty() => {
                                                if let Some(offset) = shortcut_index(key) {
                                                    let page_start = visible_page_start(selected(), suggestions().len());
                                                    let index = page_start + offset;
                                                    if index < suggestions().len() {
                                                        event.prevent_default();
                                                        selected.set(index);
                                                        selection_started.set(true);
                                                    }
                                                } else if should_exit_number_pick(key) {
                                                    number_pick_mode.set(false);
                                                }
                                            }
                                            _ => {}
                                        }
                                    },
                                    onkeyup: move |event| {
                                        let key = event.key().to_string();
                                        if key == "Tab"
                                            || key == "ArrowUp"
                                            || key == "ArrowDown"
                                            || key == "Enter"
                                            || is_space_key(&key)
                                            || (number_pick_mode() && shortcut_index(&key).is_some())
                                        {
                                            return;
                                        }
                                        if roman_enabled() {
                                            spawn(update_candidates(text(), text, roman_enabled, decoder_mode(), suggestions, popup, composition, shadow_debug, ui_status, active_token, number_pick_mode, selection_started, selected, history));
                                        }
                                    }
                                }
                                if let Some(mark) = composition() {
                                    if selection_started() {
                                        if let Some(preview) = suggestions().get(selected()).cloned() {
                                            div {
                                                class: "composition-preview",
                                                style: composition_preview_style(&mark, font_size()),
                                                span { class: "composition-preview-text", "{preview}" }
                                            }
                                        }
                                    } else {
                                        div {
                                            class: "composition-mark",
                                            style: composition_style(&mark, false),
                                        }
                                    }
                                }
                                if !suggestions().is_empty() {
                                    div {
                                        class: "suggestion-popup",
                                        "data-testid": "suggestion-popup",
                                        style: popup_style(popup()),
                                        div { class: "suggestion-popup-head", "Suggestions" }
                                        ul { class: "suggestion-list",
                                            for (index, item) in suggestions()
                                                .iter()
                                                .enumerate()
                                                .skip(visible_page_start(selected(), suggestions().len()))
                                                .take(VISIBLE_SUGGESTIONS) {
                                                li {
                                                    key: "{index}-{item}",
                                                    class: if selection_started() && index == selected() { "suggestion active" } else { "suggestion" },
                                                    button {
                                                        onclick: move |_| {
                                                            selected.set(index);
                                                            selection_started.set(true);
                                                            spawn(commit_selection(
                                                                false,
                                                                text,
                                                                suggestions,
                                                                popup,
                                                                composition,
                                                                active_token,
                                                                selection_started,
                                                                selected,
                                                                pending_caret,
                                                                history,
                                                            ));
                                                        },
                                                        span { class: "suggestion-rank", "{shortcut_label(index)}" }
                                                        span { class: "suggestion-word", "{item}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            p { class: "editor-note", "Type with roman letters, press Space to cycle or 1-5 to choose, then press Enter or Shift+Space to commit the current word." }
                        }
                    }
                    if !show_guide() {
                        button {
                            class: "guide-handle guide-handle-collapsed",
                            onclick: move |_| show_guide.set(true),
                            title: "Show typing rules",
                            ">"
                        }
                    }
                }
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
                            p { class: "eyebrow", "User guide" }
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
    }
}

fn slice_chars(input: &str, range: std::ops::Range<usize>) -> String {
    input
        .chars()
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
        .collect()
}

async fn update_candidates(
    value: String,
    live_text: Signal<String>,
    roman_enabled: Signal<bool>,
    decoder_mode: DecoderMode,
    mut suggestions: Signal<Vec<String>>,
    mut popup: Signal<Option<SuggestionPopup>>,
    mut composition: Signal<Option<CompositionMark>>,
    mut shadow_debug: Signal<Option<ShadowObservation>>,
    mut ui_status: Signal<String>,
    mut active_token: Signal<String>,
    mut number_pick_mode: Signal<bool>,
    mut selection_started: Signal<bool>,
    mut selected: Signal<usize>,
    history: Signal<HashMap<String, usize>>,
) {
    if !roman_enabled() {
        ui_status.set("roman disabled".to_owned());
        suggestions.set(Vec::new());
        popup.set(None);
        composition.set(None);
        shadow_debug.set(None);
        active_token.set(String::new());
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
        return;
    }

    let caret = current_editor_caret()
        .await
        .unwrap_or_else(|| value.chars().count());
    if live_text() != value {
        ui_status.set("stale input dropped".to_owned());
        return;
    }

    let bounds = Transliterator::token_bounds(&value, caret, false);
    let token = slice_chars(&value, bounds.clone());
    ui_status.set(format!(
        "token='{token}' caret={} decoder={:?}",
        caret, decoder_mode
    ));
    if token.trim().is_empty() {
        ui_status.set("empty token".to_owned());
        suggestions.set(Vec::new());
        popup.set(None);
        composition.set(None);
        shadow_debug.set(None);
        active_token.set(String::new());
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
        return;
    }

    let legacy = engine(DecoderMode::Legacy);
    let items = legacy.suggest(&token, &history());
    if live_text() != value {
        ui_status.set("stale after suggest".to_owned());
        return;
    }
    ui_status.set(format!(
        "token='{token}' suggestions={}",
        items.len()
    ));
    if decoder_mode != DecoderMode::Shadow {
        shadow_debug.set(None);
    }
    let preserve_selection = active_token() == token && !suggestions().is_empty();
    let popup_position = if items.is_empty() {
        None
    } else {
        suggestion_popup_position(caret).await
    };
    if live_text() != value {
        ui_status.set("stale before composition".to_owned());
        return;
    }
    let composition_mark = candidate_composition_mark(bounds.start, &token).await;
    if live_text() != value {
        ui_status.set("stale before paint".to_owned());
        return;
    }
    popup.set(popup_position);
    composition.set(composition_mark);
    active_token.set(token.clone());
    if !preserve_selection {
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
    } else if items.is_empty() {
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
    } else if selected() >= items.len() {
        selected.set(items.len().saturating_sub(1));
    }
    suggestions.set(items);
    if decoder_mode == DecoderMode::Shadow {
        ui_status.set(format!(
            "token='{token}' suggestions={} shadow=queued",
            suggestions().len()
        ));
        spawn(update_shadow_debug(
            value,
            live_text,
            history,
            shadow_debug,
            active_token,
        ));
    }
}

fn default_popup_position() -> SuggestionPopup {
    SuggestionPopup {
        left: FALLBACK_POPUP_LEFT,
        top: FALLBACK_POPUP_TOP,
    }
}

async fn suggestion_popup_position(caret: usize) -> Option<SuggestionPopup> {
    Some(
        editor_popup_position(caret)
            .await
            .unwrap_or_else(default_popup_position),
    )
}

async fn candidate_composition_mark(start: usize, token: &str) -> Option<CompositionMark> {
    editor_composition_mark(start, token).await
}

async fn update_shadow_debug(
    value: String,
    live_text: Signal<String>,
    history: Signal<HashMap<String, usize>>,
    mut shadow_debug: Signal<Option<ShadowObservation>>,
    active_token: Signal<String>,
) {
    if live_text() != value {
        return;
    }
    let token = active_token();
    if token.trim().is_empty() {
        shadow_debug.set(None);
        return;
    }
    let shadow = engine(DecoderMode::Shadow);
    let observation = shadow.shadow_observation(&token, &history());
    if live_text() != value || active_token() != token {
        return;
    }
    shadow_debug.set(Some(observation));
}

async fn commit_selection(
    typed_space: bool,
    mut text: Signal<String>,
    mut suggestions: Signal<Vec<String>>,
    mut popup: Signal<Option<SuggestionPopup>>,
    mut composition: Signal<Option<CompositionMark>>,
    mut active_token: Signal<String>,
    mut selection_started: Signal<bool>,
    mut selected: Signal<usize>,
    mut pending_caret: Signal<Option<usize>>,
    mut history: Signal<HashMap<String, usize>>,
) {
    let items = suggestions();
    if items.is_empty() {
        return;
    }
    let Some(choice) = items.get(selected()).cloned() else {
        return;
    };
    let current_text = text();
    let caret = current_editor_caret()
        .await
        .unwrap_or_else(|| current_text.chars().count());
    let applied = Transliterator::apply_suggestion(&current_text, caret, &choice, typed_space);

    let mut next_history = history();
    Transliterator::learn(&mut next_history, &choice);
    save_history(&next_history);
    history.set(next_history);

    save_editor_text(&applied.text);
    text.set(applied.text);
    suggestions.set(Vec::new());
    popup.set(None);
    composition.set(None);
    active_token.set(String::new());
    selection_started.set(false);
    selected.set(0);
    pending_caret.set(Some(applied.caret));
}

async fn refresh_popup_position(mut popup: Signal<Option<SuggestionPopup>>) {
    let Some(caret) = current_editor_caret().await else {
        popup.set(None);
        return;
    };
    popup.set(suggestion_popup_position(caret).await);
}

fn popup_style(popup: Option<SuggestionPopup>) -> String {
    let Some(popup) = popup else {
        return "display:none;".to_owned();
    };
    format!("left:{:.1}px; top:{:.1}px;", popup.left, popup.top)
}

fn composition_style(mark: &CompositionMark, selection_started: bool) -> String {
    let top = mark.top + mark.height - 3.0;
    let opacity = if selection_started { 0.75 } else { 1.0 };
    format!(
        "left:{:.1}px; top:{:.1}px; width:{:.1}px; opacity:{:.2};",
        mark.left, top, mark.width, opacity
    )
}

fn composition_preview_style(mark: &CompositionMark, font_size: usize) -> String {
    format!(
        "left:{:.1}px; top:{:.1}px; width:{:.1}px; height:{:.1}px; font-size:{}px;",
        mark.left, mark.top, mark.width, mark.height, font_size
    )
}

fn shortcut_index(key: &str) -> Option<usize> {
    match key {
        "1" => Some(0),
        "2" => Some(1),
        "3" => Some(2),
        "4" => Some(3),
        "5" => Some(4),
        _ => None,
    }
}

fn shortcut_label(index: usize) -> String {
    ((index % VISIBLE_SUGGESTIONS) + 1).to_string()
}

fn visible_page_start(selected: usize, total: usize) -> usize {
    if total <= VISIBLE_SUGGESTIONS {
        0
    } else {
        (selected / VISIBLE_SUGGESTIONS) * VISIBLE_SUGGESTIONS
    }
}

fn should_exit_number_pick(key: &str) -> bool {
    matches!(key, "Backspace" | "Delete" | "ArrowLeft" | "ArrowRight" | "Escape") || key.chars().count() == 1
}

fn is_space_key(key: &str) -> bool {
    matches!(key, " " | "Space" | "Spacebar")
}

#[cfg(target_arch = "wasm32")]
async fn editor_composition_mark(start: usize, token: &str) -> Option<CompositionMark> {
    let script = format!(
        r#"
            const el = document.getElementById({editor_id:?});
            if (!el) return "";

            const style = window.getComputedStyle(el);
            const mirror = document.createElement("div");
            const props = [
                "boxSizing", "width", "paddingTop", "paddingRight", "paddingBottom", "paddingLeft",
                "borderTopWidth", "borderRightWidth", "borderBottomWidth", "borderLeftWidth",
                "fontFamily", "fontSize", "fontWeight", "fontStyle", "lineHeight",
                "letterSpacing", "textTransform", "textIndent", "whiteSpace", "wordSpacing"
            ];

            mirror.style.position = "absolute";
            mirror.style.visibility = "hidden";
            mirror.style.whiteSpace = "pre-wrap";
            mirror.style.wordWrap = "break-word";
            mirror.style.left = "-9999px";
            mirror.style.top = "0";

            for (const prop of props) {{
                mirror.style[prop] = style[prop];
            }}

            mirror.style.width = `${{el.clientWidth}}px`;

            const raw = el.value || "";
            mirror.textContent = raw.slice(0, Math.min({start}, raw.length));

            const marker = document.createElement("span");
            marker.textContent = {token:?};
            mirror.appendChild(marker);
            document.body.appendChild(mirror);

            const mirrorRect = mirror.getBoundingClientRect();
            const markerRect = marker.getBoundingClientRect();
            document.body.removeChild(mirror);

            const left = markerRect.left - mirrorRect.left - el.scrollLeft;
            const top = markerRect.top - mirrorRect.top - el.scrollTop;
            return `${{left}},${{top}},${{markerRect.width}},${{markerRect.height}}`;
        "#,
        editor_id = EDITOR_ID,
        start = start,
        token = token,
    );
    let raw = document::eval(&script).join::<String>().await.ok()?;
    let mut parts = raw.split(',');
    let left = parts.next()?;
    let top = parts.next()?;
    let width = parts.next()?;
    let height = parts.next()?;
    Some(CompositionMark {
        left: left.trim().parse().ok()?,
        top: top.trim().parse().ok()?,
        width: width.trim().parse().ok()?,
        height: height.trim().parse().ok()?,
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn editor_composition_mark(start: usize, token: &str) -> Option<CompositionMark> {
    let script = format!(
        r#"
            const el = document.getElementById({editor_id:?});
            if (!el) return "";
            const style = getComputedStyle(el);
            const fontSize = parseFloat(style.fontSize) || 24;
            const lineHeight = parseFloat(style.lineHeight) || fontSize * 1.5;
            const charsPerLine = Math.max(1, Math.floor(el.clientWidth / (fontSize * 0.62)));
            let left = ({start} % charsPerLine) * (fontSize * 0.58);
            let top = Math.floor({start} / charsPerLine) * lineHeight;
            const width = Math.max(12, ({token_len} * fontSize * 0.58));
            return `${{left}},${{top}},${{width}},${{lineHeight}}`;
        "#,
        editor_id = EDITOR_ID,
        start = start,
        token_len = token.chars().count(),
    );
    let raw = document::eval(&script).join::<String>().await.ok()?;
    let mut parts = raw.split(',');
    let left = parts.next()?;
    let top = parts.next()?;
    let width = parts.next()?;
    let height = parts.next()?;
    Some(CompositionMark {
        left: left.trim().parse().ok()?,
        top: top.trim().parse().ok()?,
        width: width.trim().parse().ok()?,
        height: height.trim().parse().ok()?,
    })
}

#[cfg(target_arch = "wasm32")]
async fn editor_popup_position(caret: usize) -> Option<SuggestionPopup> {
    let script = format!(
        r#"
            const el = document.getElementById({editor_id:?});
            if (!el) return "";

            const style = window.getComputedStyle(el);
            const mirror = document.createElement("div");
            const props = [
                "boxSizing", "width", "paddingTop", "paddingRight", "paddingBottom", "paddingLeft",
                "borderTopWidth", "borderRightWidth", "borderBottomWidth", "borderLeftWidth",
                "fontFamily", "fontSize", "fontWeight", "fontStyle", "lineHeight",
                "letterSpacing", "textTransform", "textIndent", "whiteSpace", "wordSpacing"
            ];

            mirror.style.position = "absolute";
            mirror.style.visibility = "hidden";
            mirror.style.whiteSpace = "pre-wrap";
            mirror.style.wordWrap = "break-word";
            mirror.style.left = "-9999px";
            mirror.style.top = "0";

            for (const prop of props) {{
                mirror.style[prop] = style[prop];
            }}

            mirror.style.width = `${{el.clientWidth}}px`;

            const raw = el.value || "";
            const index = Math.min({caret}, raw.length);
            mirror.textContent = raw.slice(0, index);

            const marker = document.createElement("span");
            marker.textContent = raw.slice(index, index + 1) || ".";
            mirror.appendChild(marker);
            document.body.appendChild(mirror);

            const mirrorRect = mirror.getBoundingClientRect();
            const markerRect = marker.getBoundingClientRect();
            const lineHeight = parseFloat(style.lineHeight) || parseFloat(style.fontSize) * 1.5 || 32;

            let left = markerRect.left - mirrorRect.left - el.scrollLeft + 18;
            let top = markerRect.top - mirrorRect.top - el.scrollTop + lineHeight + 10;

            left = Math.max(10, Math.min(left, el.clientWidth - 250));
            top = Math.max(10, Math.min(top, el.clientHeight - 220));

            document.body.removeChild(mirror);
            return `${{left}},${{top}}`;
        "#,
        editor_id = EDITOR_ID,
        caret = caret,
    );
    let raw = document::eval(&script).join::<String>().await.ok()?;
    parse_popup_position(&raw)
}

#[cfg(not(target_arch = "wasm32"))]
async fn editor_popup_position(caret: usize) -> Option<SuggestionPopup> {
    let script = format!(
        r#"
            const el = document.getElementById({editor_id:?});
            if (!el) return "";
            const style = getComputedStyle(el);
            const fontSize = parseFloat(style.fontSize) || 24;
            const lineHeight = parseFloat(style.lineHeight) || fontSize * 1.5;
            const charsPerLine = Math.max(1, Math.floor(el.clientWidth / (fontSize * 0.62)));
            let left = 18 + (({caret} % charsPerLine) * (fontSize * 0.58));
            let top = 18 + (Math.floor({caret} / charsPerLine) * lineHeight) + lineHeight;
            left = Math.max(10, Math.min(left, el.clientWidth - 250));
            top = Math.max(10, Math.min(top, el.clientHeight - 220));
            return `${{left}},${{top}}`;
        "#,
        editor_id = EDITOR_ID,
        caret = caret,
    );
    let raw = document::eval(&script).join::<String>().await.ok()?;
    parse_popup_position(&raw)
}

fn parse_popup_position(raw: &str) -> Option<SuggestionPopup> {
    let (left, top) = raw.split_once(',')?;
    Some(SuggestionPopup {
        left: left.trim().parse().ok()?,
        top: top.trim().parse().ok()?,
    })
}

#[cfg(target_arch = "wasm32")]
async fn current_editor_caret() -> Option<usize> {
    let document = window()?.document()?;
    let editor = document
        .get_element_by_id(EDITOR_ID)?
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .ok()?;
    editor.selection_start().ok().flatten().map(|idx| idx as usize)
}

#[cfg(not(target_arch = "wasm32"))]
async fn current_editor_caret() -> Option<usize> {
    let script = format!(
        r#"
            const el = document.getElementById({editor_id:?});
            if (!el) return 0;
            return typeof el.selectionStart === "number" ? el.selectionStart : (el.value ? el.value.length : 0);
        "#,
        editor_id = EDITOR_ID,
    );
    document::eval(&script).join::<usize>().await.ok()
}

#[cfg(target_arch = "wasm32")]
fn set_editor_caret(caret: usize) {
    let Some(document) = window().and_then(|w| w.document()) else {
        return;
    };
    let Some(element) = document.get_element_by_id(EDITOR_ID) else {
        return;
    };
    let Ok(editor) = element.dyn_into::<web_sys::HtmlTextAreaElement>() else {
        return;
    };
    let cursor = caret.min(editor.value().chars().count()) as u32;
    let _ = editor.focus();
    let _ = editor.set_selection_range(cursor, cursor);
}

#[cfg(not(target_arch = "wasm32"))]
fn set_editor_caret(caret: usize) {
    let script = format!(
        r#"
            const el = document.getElementById({editor_id:?});
            if (el) {{
                el.focus();
                if (typeof el.setSelectionRange === "function") {{
                    el.setSelectionRange({caret}, {caret});
                }}
            }}
        "#,
        editor_id = EDITOR_ID,
        caret = caret,
    );
    document::eval(&script);
}

#[cfg(target_arch = "wasm32")]
fn load_editor_text() -> String {
    storage_get_web(STORAGE_TEXT).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn load_editor_text() -> String {
    String::new()
}

#[cfg(target_arch = "wasm32")]
fn save_editor_text(value: &str) {
    let _ = storage_set_web(STORAGE_TEXT, value);
}

#[cfg(not(target_arch = "wasm32"))]
fn save_editor_text(_: &str) {}

#[cfg(target_arch = "wasm32")]
fn load_enabled() -> bool {
    storage_get_web(STORAGE_ENABLED)
        .map(|value| value != "0")
        .unwrap_or(true)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_enabled() -> bool {
    true
}

#[cfg(target_arch = "wasm32")]
fn save_enabled(value: bool) {
    let _ = storage_set_web(STORAGE_ENABLED, if value { "1" } else { "0" });
}

#[cfg(not(target_arch = "wasm32"))]
fn save_enabled(_: bool) {}

#[cfg(target_arch = "wasm32")]
fn load_decoder_mode() -> DecoderMode {
    match storage_get_web(STORAGE_DECODER_MODE).as_deref() {
        Some("shadow") => DecoderMode::Shadow,
        _ => DecoderMode::Legacy,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_decoder_mode() -> DecoderMode {
    DecoderMode::Legacy
}

#[cfg(target_arch = "wasm32")]
fn save_decoder_mode(mode: DecoderMode) {
    let value = match mode {
        DecoderMode::Shadow => "shadow",
        _ => "legacy",
    };
    let _ = storage_set_web(STORAGE_DECODER_MODE, value);
}

#[cfg(not(target_arch = "wasm32"))]
fn save_decoder_mode(_: DecoderMode) {}

#[cfg(target_arch = "wasm32")]
fn load_history() -> HashMap<String, usize> {
    storage_get_web(STORAGE_HISTORY)
        .map(|raw| {
            raw.lines()
                .filter_map(|line| {
                    let (word, count) = line.split_once('\t')?;
                    Some((word.to_owned(), count.parse().ok()?))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn load_history() -> HashMap<String, usize> {
    HashMap::new()
}

#[cfg(target_arch = "wasm32")]
fn save_history(history: &HashMap<String, usize>) {
    let mut rows = history
        .iter()
        .map(|(word, count)| format!("{word}\t{count}"))
        .collect::<Vec<_>>();
    rows.sort();
    let _ = storage_set_web(STORAGE_HISTORY, &rows.join("\n"));
}

#[cfg(not(target_arch = "wasm32"))]
fn save_history(_: &HashMap<String, usize>) {}

#[cfg(target_arch = "wasm32")]
fn load_font_size() -> usize {
    storage_get_web(STORAGE_FONT_SIZE)
        .and_then(|value| value.parse::<usize>().ok())
        .map(|value| value.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE))
        .unwrap_or(DEFAULT_FONT_SIZE)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_font_size() -> usize {
    DEFAULT_FONT_SIZE
}

#[cfg(target_arch = "wasm32")]
fn save_font_size(value: usize) {
    let _ = storage_set_web(
        STORAGE_FONT_SIZE,
        &value.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE).to_string(),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn save_font_size(_: usize) {}

#[cfg(target_arch = "wasm32")]
fn storage_get_web(key: &str) -> Option<String> {
    let storage = window()?.local_storage().ok().flatten()?;
    storage.get_item(key).ok().flatten()
}

#[cfg(target_arch = "wasm32")]
fn storage_set_web(key: &str, value: &str) -> Option<()> {
    let storage = window()?.local_storage().ok().flatten()?;
    storage.set_item(key, value).ok()?;
    Some(())
}
