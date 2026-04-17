use std::collections::HashMap;
use std::sync::OnceLock;

use dioxus::document;
use dioxus::prelude::*;
use roman_lookup::{DecoderConfig, DecoderMode, ShadowObservation, Transliterator};

mod ui;

use self::ui::components::{AppToolbar, EditorCard, GuidePanel, WorkspaceBody};
use self::ui::editor::{refresh_popup_position, EditorSignals, SegmentedSession};
use self::ui::platform::{mark_app_ready, set_editor_caret};
use self::ui::storage::{load_decoder_mode, load_editor_text, load_enabled, load_font_size, load_history};

const APP_CSS: &str = include_str!("../assets/main.css");
static LEGACY_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
static SHADOW_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
static WFST_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
static HYBRID_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static LEXICON_DATA: OnceLock<Vec<u8>> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static KHPOS_DATA: OnceLock<Vec<u8>> = OnceLock::new();

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
    #[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
    macro_rules! make_transliterator {
        ($config:expr) => {{
            let lexicon = LEXICON_DATA
                .get()
                .expect("lexicon data must be fetched before engine init");
            let khpos = KHPOS_DATA.get().expect("khpos data must be fetched before engine init");
            Transliterator::from_compiled_bytes(lexicon, khpos, $config).expect("transliterator init failed")
        }};
    }
    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    macro_rules! make_transliterator {
        ($config:expr) => {{
            Transliterator::from_default_data_with_config($config).expect("embedded lexicon must load")
        }};
    }
    match mode {
        DecoderMode::Legacy => LEGACY_TRANSLITERATOR.get_or_init(|| make_transliterator!(DecoderConfig::default())),
        DecoderMode::Shadow => SHADOW_TRANSLITERATOR
            .get_or_init(|| make_transliterator!(DecoderConfig::shadow_interactive().with_shadow_log(false))),
        DecoderMode::Wfst => WFST_TRANSLITERATOR.get_or_init(|| {
            make_transliterator!(DecoderConfig::default()
                .with_mode(DecoderMode::Wfst)
                .with_shadow_log(false))
        }),
        DecoderMode::Hybrid => HYBRID_TRANSLITERATOR.get_or_init(|| {
            make_transliterator!(DecoderConfig::default()
                .with_mode(DecoderMode::Hybrid)
                .with_shadow_log(false))
        }),
    }
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
async fn fetch_binaries_parallel() -> Result<(Vec<u8>, Vec<u8>), String> {
    use js_sys::wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;

    let lexicon_url = "./assets/data/roman_lookup.lexicon.bin";
    let khpos_url = "./assets/data/khpos.stats.bin";

    let window = web_sys::window().ok_or_else(|| "no window".to_owned())?;
    // Start both requests first so network transfer happens in parallel.
    let lexicon_promise = window.fetch_with_str(lexicon_url);
    let khpos_promise = window.fetch_with_str(khpos_url);

    let lexicon_resp = JsFuture::from(lexicon_promise)
        .await
        .map_err(|e| format!("fetch {lexicon_url} network error: {e:?}"))?;
    let khpos_resp = JsFuture::from(khpos_promise)
        .await
        .map_err(|e| format!("fetch {khpos_url} network error: {e:?}"))?;

    let lexicon_resp: Response = lexicon_resp
        .dyn_into()
        .map_err(|_| format!("fetch {lexicon_url}: response cast failed"))?;
    let khpos_resp: Response = khpos_resp
        .dyn_into()
        .map_err(|_| format!("fetch {khpos_url}: response cast failed"))?;

    if !lexicon_resp.ok() {
        return Err(format!("fetch {lexicon_url}: HTTP {}", lexicon_resp.status()));
    }
    if !khpos_resp.ok() {
        return Err(format!("fetch {khpos_url}: HTTP {}", khpos_resp.status()));
    }

    let lexicon_buf = JsFuture::from(
        lexicon_resp
            .array_buffer()
            .map_err(|_| format!("fetch {lexicon_url}: array_buffer() failed"))?,
    )
    .await
    .map_err(|e| format!("fetch {lexicon_url}: array_buffer await failed: {e:?}"))?;
    let khpos_buf = JsFuture::from(
        khpos_resp
            .array_buffer()
            .map_err(|_| format!("fetch {khpos_url}: array_buffer() failed"))?,
    )
    .await
    .map_err(|e| format!("fetch {khpos_url}: array_buffer await failed: {e:?}"))?;

    Ok((
        js_sys::Uint8Array::new(&lexicon_buf).to_vec(),
        js_sys::Uint8Array::new(&khpos_buf).to_vec(),
    ))
}

fn warm_engine(mode: DecoderMode) {
    let transliterator = engine(mode);
    let history = HashMap::new();
    let _ = transliterator.suggest("a", &history);
    if mode == DecoderMode::Shadow {
        let _ = transliterator.shadow_observation("khn", &history);
    }
}

#[component]
fn App() -> Element {
    let mut engine_ready = use_signal(|| LEGACY_TRANSLITERATOR.get().is_some());
    let mut engine_progress = use_signal(|| if LEGACY_TRANSLITERATOR.get().is_some() { 100 } else { 0 });
    let text = use_signal(load_editor_text);
    let roman_enabled = use_signal(load_enabled);
    let decoder_mode = use_signal(load_decoder_mode);
    let font_size = use_signal(|| load_font_size(MIN_FONT_SIZE, MAX_FONT_SIZE, DEFAULT_FONT_SIZE));
    let show_guide = use_signal(|| false);
    let suggestions = use_signal(Vec::<String>::new);
    let mut popup = use_signal(|| None::<SuggestionPopup>);
    let composition = use_signal(|| None::<CompositionMark>);
    let shadow_debug = use_signal(|| None::<ShadowObservation>);
    let segmented_session = use_signal(|| None::<SegmentedSession>);
    let segmented_refine_mode = use_signal(|| false);
    let active_token = use_signal(String::new);
    let recommended_indices = use_signal(Vec::<usize>::new);
    let roman_variant_hints = use_signal(HashMap::<usize, Vec<String>>::new);
    let mut number_pick_mode = use_signal(|| false);
    let mut selection_started = use_signal(|| false);
    let selected = use_signal(|| 0usize);
    let mut pending_caret = use_signal(|| None::<usize>);
    let history = use_signal(load_history);
    let editor_state = EditorSignals {
        text,
        roman_enabled,
        decoder_mode,
        engine_ready,
        engine_progress,
        suggestions,
        popup,
        composition,
        shadow_debug,
        segmented_session,
        segmented_refine_mode,
        active_token,
        recommended_indices,
        roman_variant_hints,
        number_pick_mode,
        selection_started,
        selected,
        pending_caret,
        history,
    };

    use_effect(move || {
        if let Some(caret) = pending_caret() {
            set_editor_caret(caret);
            pending_caret.set(None);
        }
    });

    use_effect(move || {
        spawn(async move {
            if engine_ready() {
                engine_progress.set(100);
                return;
            }

            engine_progress.set(10);
            #[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
            {
                engine_progress.set(35);
                match fetch_binaries_parallel().await {
                    Ok((lexicon, khpos)) => {
                        let _ = LEXICON_DATA.set(lexicon);
                        let _ = KHPOS_DATA.set(khpos);
                        engine_progress.set(70);
                    }
                    Err(e) => {
                        web_sys::console::error_1(&e.into());
                        engine_progress.set(100);
                        engine_ready.set(true); // unblock UI even on data load failure
                        return;
                    }
                }
            }
            engine_progress.set(85);
            warm_engine(DecoderMode::Legacy);
            engine_progress.set(100);
            engine_ready.set(true);
        });
    });

    use_effect(move || {
        if engine_ready() {
            mark_app_ready();
        }
    });

    use_effect(move || {
        if editor_state.engine_ready() && editor_state.roman_enabled() {
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
