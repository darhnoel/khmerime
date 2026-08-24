//! Dioxus web/desktop shell for KhmerIME.
//!
//! This app owns browser/desktop UI state, storage, caret/popup positioning, and
//! startup loading. Transliteration, segmentation, and ranking remain in the
//! shared engine crates so native adapters and CLI tools see the same behavior.

use std::collections::HashMap;

use dioxus::prelude::*;
use roman_lookup::ShadowObservation;

mod engine_registry;
mod startup_fetch;
mod startup_signals;
mod ui;

pub(crate) use self::engine_registry::{engine, EngineReadiness};

use self::engine_registry::current_engine_readiness;
use self::startup_fetch::start_engine_bootstrap;
use self::startup_signals::StartupSignals;
use self::ui::components::{AppToolbar, EditorCard, GuidePanel, WorkspaceBody};
use self::ui::editor::{refresh_popup_position, CandidateLevel, CandidateMode, EditorSignals, SegmentedSession};
use self::ui::platform::{mark_app_ready, mark_app_shell_ready, refresh_mobile_layout_density, set_editor_caret};
use self::ui::storage::{
    load_decoder_mode, load_editor_text, load_enabled, load_font_size, load_history, load_sidebar_open, load_theme,
    load_user_dictionary, save_theme,
};

pub(crate) const EDITOR_ID: &str = "ime-editor";
// The stylesheet is EMBEDDED, not fetched. dx serve does not statically serve
// asset_dir files, so an external `href: "/assets/main.css"` 404s to the SPA
// index and no styles load (doubled toolbar labels, no spacing). Inline the
// ordered partials via include_str! so the CSS ships in the binary. Keep this
// list in sync with the @imports in assets/main.css.
const APP_CSS: &str = concat!(
    include_str!("../../../assets/css/00-tokens.css"),
    include_str!("../../../assets/css/01-base.css"),
    include_str!("../../../assets/css/10-layout.css"),
    include_str!("../../../assets/css/20-toolbar.css"),
    include_str!("../../../assets/css/30-editor.css"),
    include_str!("../../../assets/css/40-candidates.css"),
    include_str!("../../../assets/css/50-guide-debug.css"),
    include_str!("../../../assets/css/90-responsive.css"),
);

const DEFAULT_FONT_SIZE: usize = 24;
pub(crate) const MIN_FONT_SIZE: usize = 18;
pub(crate) const MAX_FONT_SIZE: usize = 38;
pub(crate) const VISIBLE_SUGGESTIONS: usize = 5;
pub(crate) const FALLBACK_POPUP_LEFT: f64 = 18.0;
pub(crate) const FALLBACK_POPUP_TOP: f64 = 88.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SuggestionPopup {
    left: f64,
    top: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CompositionMark {
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

#[component]
fn App() -> Element {
    let initial_readiness = current_engine_readiness();
    let engine_readiness = use_signal(|| initial_readiness);
    let engine_ready = use_signal(|| initial_readiness.is_ready());
    let engine_progress = use_signal(|| if initial_readiness.is_ready() { 100u8 } else { 0u8 });
    let mut startup_started = use_signal(|| false);
    let text = use_signal(load_editor_text);
    let roman_enabled = use_signal(load_enabled);
    let decoder_mode = use_signal(load_decoder_mode);
    let font_size = use_signal(|| load_font_size(MIN_FONT_SIZE, MAX_FONT_SIZE, DEFAULT_FONT_SIZE));
    let theme = use_signal(load_theme);
    // Apply + persist the theme by stamping data-theme on <html>. No attribute
    // means System, so CSS can follow prefers-color-scheme.
    use_effect(move || {
        let selected = theme();
        let attr = selected.data_attr();
        save_theme(selected);
        let script = match attr {
            None => "document.documentElement.removeAttribute('data-theme');".to_string(),
            Some(attr) => format!("document.documentElement.setAttribute('data-theme', {attr:?});"),
        };
        let _ = dioxus::document::eval(&script);
    });
    let show_guide = use_signal(|| false);
    let sidebar_open = use_signal(load_sidebar_open);

    use_effect(move || {
        let _ = dioxus::document::eval(
            r#"
            (() => {
                if (window.__khmerImeGuideEscapeInstalled) return;
                window.__khmerImeGuideEscapeInstalled = true;
                document.addEventListener("keydown", (event) => {
                    if (event.key === "Escape") {
                        document.querySelector('[data-testid="close-rules"]')?.click();
                    }
                });
            })();
            "#,
        );
    });

    use_effect(move || {
        let script = r#"
            requestAnimationFrame(() => {
                const root = document.documentElement;
                root.dataset.scrollCue = root.scrollHeight > window.innerHeight + 24 ? '1' : '0';
                window.addEventListener('scroll', () => { root.dataset.scrollCue = '0'; }, { once: true, passive: true });
            });
        "#;
        let _ = dioxus::document::eval(script);
    });
    let suggestions = use_signal(Vec::<String>::new);
    let mut popup = use_signal(|| None::<SuggestionPopup>);
    let composition = use_signal(|| None::<CompositionMark>);
    let shadow_debug = use_signal(|| None::<ShadowObservation>);
    let segmented_session = use_signal(|| None::<SegmentedSession>);
    let segmented_refine_mode = use_signal(|| false);
    let phrase_candidates = use_signal(Vec::<roman_lookup::DecodeCandidate>::new);
    let candidate_level = use_signal(CandidateLevel::default);
    let active_phrase_index = use_signal(|| 0usize);
    let suggestion_loading = use_signal(|| false);
    let suggestion_request_id = use_signal(|| 0u64);
    let candidate_mode = use_signal(|| CandidateMode::None);
    let active_token = use_signal(String::new);
    let recommended_indices = use_signal(Vec::<usize>::new);
    let roman_variant_hints = use_signal(HashMap::<usize, Vec<String>>::new);
    let mut number_pick_mode = use_signal(|| false);
    let mut selection_started = use_signal(|| false);
    let selected = use_signal(|| 0usize);
    let mut pending_caret = use_signal(|| None::<usize>);
    let history = use_signal(load_history);
    let user_dictionary = use_signal(load_user_dictionary);
    let editor_state = EditorSignals {
        text,
        roman_enabled,
        decoder_mode,
        engine_readiness,
        engine_ready,
        engine_progress,
        suggestions,
        popup,
        composition,
        shadow_debug,
        segmented_session,
        segmented_refine_mode,
        phrase_candidates,
        candidate_level,
        active_phrase_index,
        suggestion_loading,
        suggestion_request_id,
        candidate_mode,
        active_token,
        recommended_indices,
        roman_variant_hints,
        number_pick_mode,
        selection_started,
        selected,
        pending_caret,
        history,
        user_dictionary,
    };

    use_effect(move || {
        if let Some(caret) = pending_caret() {
            set_editor_caret(caret);
            pending_caret.set(None);
        }
    });

    use_effect(move || {
        mark_app_shell_ready();
        refresh_mobile_layout_density();
    });

    use_effect(move || {
        if startup_started() {
            return;
        }
        startup_started.set(true);
        start_engine_bootstrap(StartupSignals {
            engine_readiness,
            engine_ready,
            engine_progress,
        });
    });

    use_effect(move || {
        if engine_ready() {
            mark_app_ready();
            refresh_mobile_layout_density();
        }
    });

    use_effect(move || {
        let _ = suggestions().len();
        let _ = font_size();
        let _ = segmented_refine_mode();
        refresh_mobile_layout_density();
    });

    use_effect(move || {
        if editor_state.roman_enabled() && editor_state.engine_ready() {
            spawn(ui::editor::update_candidates(editor_state.text(), editor_state));
        }
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
        // Embedded stylesheet (see APP_CSS) — injected inline so it always loads.
        style { {APP_CSS} }
        div { class: "shell",
            div { class: "board",
                section { class: if sidebar_open() { "workspace sidebar-open" } else { "workspace sidebar-closed" },
                    AppToolbar {
                        state: editor_state,
                        show_guide,
                        font_size,
                        theme,
                        sidebar_open,
                    }
                    WorkspaceBody {
                        roman_enabled: editor_state.roman_enabled(),
                        decoder_mode: editor_state.decoder_mode(),
                        shadow_debug: editor_state.shadow_debug(),
                        editor_card: rsx! {
                            EditorCard {
                                state: editor_state,
                                font_size,
                            }
                        },
                    }
                }
                GuidePanel { show_guide }
            }
            div { class: "scroll-cue", aria_hidden: "true",
                span { "⌄" }
                span { "រំកិលចុះ" }
            }
        }
    }
}
