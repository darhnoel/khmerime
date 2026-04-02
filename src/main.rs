use std::sync::OnceLock;

use dioxus::document;
use dioxus::prelude::*;
use roman_lookup::{DecoderConfig, DecoderMode, ShadowObservation, Transliterator};

mod ui;

use self::ui::components::{AppToolbar, EditorCard, GuidePanel, WorkspaceBody};
use self::ui::editor::{
    refresh_popup_position,
};
use self::ui::platform::set_editor_caret;
use self::ui::storage::{
    load_decoder_mode, load_editor_text, load_enabled, load_font_size, load_history,
};

static STYLES: Asset = asset!("/assets/main.css");
static LEGACY_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
static SHADOW_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();

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

pub(crate) fn engine(mode: DecoderMode) -> &'static Transliterator {
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
    let text = use_signal(load_editor_text);
    let roman_enabled = use_signal(load_enabled);
    let decoder_mode = use_signal(load_decoder_mode);
    let font_size = use_signal(|| load_font_size(MIN_FONT_SIZE, MAX_FONT_SIZE, DEFAULT_FONT_SIZE));
    let show_guide = use_signal(|| false);
    let suggestions = use_signal(Vec::<String>::new);
    let mut popup = use_signal(|| None::<SuggestionPopup>);
    let composition = use_signal(|| None::<CompositionMark>);
    let shadow_debug = use_signal(|| None::<ShadowObservation>);
    let active_token = use_signal(String::new);
    let mut number_pick_mode = use_signal(|| false);
    let mut selection_started = use_signal(|| false);
    let selected = use_signal(|| 0usize);
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
                    AppToolbar {
                        text,
                        roman_enabled,
                        decoder_mode,
                        font_size,
                        suggestions,
                        popup,
                        composition,
                        shadow_debug,
                        active_token,
                        number_pick_mode,
                        selection_started,
                        selected,
                        pending_caret,
                        history,
                    }
                    WorkspaceBody {
                        roman_enabled: roman_enabled(),
                        decoder_mode: decoder_mode(),
                        shadow_debug: shadow_debug(),
                        editor_card: rsx! {
                            EditorCard {
                                text,
                                roman_enabled,
                                decoder_mode,
                                font_size,
                                suggestions,
                                popup,
                                composition,
                                shadow_debug,
                                active_token,
                                number_pick_mode,
                                selection_started,
                                selected,
                                pending_caret,
                                history,
                            }
                        },
                    }
                }
                GuidePanel { show_guide }
            }
        }
    }
}
