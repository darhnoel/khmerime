use std::collections::HashMap;

use dioxus::document;
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
use self::ui::editor::{
    refresh_popup_position, CandidateMode, EditorSignals, InputMode, ManualSaveRequest, ManualTypingState,
    SegmentedSession,
};
use self::ui::platform::{mark_app_ready, mark_app_shell_ready, refresh_mobile_layout_density, set_editor_caret};
use self::ui::storage::{
    load_decoder_mode, load_editor_text, load_enabled, load_font_size, load_history, load_user_dictionary,
};

const APP_CSS: &str = include_str!("../assets/main.css");

pub(crate) const EDITOR_ID: &str = "ime-editor";
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
    let input_mode = use_signal(|| InputMode::NormalWordSuggestion);
    let decoder_mode = use_signal(load_decoder_mode);
    let font_size = use_signal(|| load_font_size(MIN_FONT_SIZE, MAX_FONT_SIZE, DEFAULT_FONT_SIZE));
    let show_guide = use_signal(|| false);
    let suggestions = use_signal(Vec::<String>::new);
    let mut popup = use_signal(|| None::<SuggestionPopup>);
    let composition = use_signal(|| None::<CompositionMark>);
    let shadow_debug = use_signal(|| None::<ShadowObservation>);
    let segmented_session = use_signal(|| None::<SegmentedSession>);
    let segmented_refine_mode = use_signal(|| false);
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
    let manual_typing_state = use_signal(|| None::<ManualTypingState>);
    let manual_save_request = use_signal(|| None::<ManualSaveRequest>);
    let user_dictionary = use_signal(load_user_dictionary);
    let editor_state = EditorSignals {
        text,
        roman_enabled,
        input_mode,
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
        manual_typing_state,
        manual_save_request,
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
        let _ = input_mode();
        let _ = font_size();
        let _ = segmented_refine_mode();
        refresh_mobile_layout_density();
    });

    use_effect(move || {
        if editor_state.roman_enabled()
            && (editor_state.engine_ready() || editor_state.input_mode() == InputMode::ManualCharacterTyping)
        {
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
        document::Style { {APP_CSS} }
        div { class: "shell",
            div { class: if show_guide() { "board" } else { "board board-wide" },
                section { class: "workspace",
                    AppToolbar {
                        state: editor_state,
                        show_guide,
                        font_size,
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
        }
    }
}
