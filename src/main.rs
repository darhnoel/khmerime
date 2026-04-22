use std::collections::HashMap;
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use dioxus::document;
use dioxus::prelude::*;
use roman_lookup::{DecoderConfig, DecoderMode, ShadowObservation, Transliterator};

mod ui;

use self::ui::components::{AppToolbar, EditorCard, GuidePanel, WorkspaceBody};
use self::ui::editor::{
    refresh_popup_position, EditorSignals, InputMode, ManualSaveRequest, ManualTypingState, SegmentedSession,
};
use self::ui::platform::{mark_app_ready, mark_app_shell_ready, refresh_mobile_layout_density, set_editor_caret};
use self::ui::storage::{
    load_decoder_mode, load_editor_text, load_enabled, load_font_size, load_history, load_user_dictionary,
};

const APP_CSS: &str = include_str!("../assets/main.css");
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static PHASE_A_LEGACY_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static FULL_LEGACY_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static FULL_SHADOW_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static FULL_WFST_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static FULL_HYBRID_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static FULL_ENGINE_READY: AtomicBool = AtomicBool::new(false);

#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
static LEGACY_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
static SHADOW_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
static WFST_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EngineReadiness {
    Booting,
    LegacyReady,
    FullReady,
    Failed,
}

impl EngineReadiness {
    fn is_ready(self) -> bool {
        matches!(
            self,
            EngineReadiness::LegacyReady | EngineReadiness::FullReady | EngineReadiness::Failed
        )
    }
}

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
    {
        if FULL_ENGINE_READY.load(Ordering::Acquire) {
            return full_engine(mode);
        }
        return PHASE_A_LEGACY_TRANSLITERATOR.get_or_init(init_phase_a_legacy_transliterator);
    }
    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    {
        match mode {
            DecoderMode::Legacy => LEGACY_TRANSLITERATOR.get_or_init(|| {
                Transliterator::from_default_data_with_config(DecoderConfig::default())
                    .expect("embedded lexicon must load")
            }),
            DecoderMode::Shadow => SHADOW_TRANSLITERATOR.get_or_init(|| {
                Transliterator::from_default_data_with_config(
                    DecoderConfig::shadow_interactive().with_shadow_log(false),
                )
                .expect("embedded lexicon must load")
            }),
            DecoderMode::Wfst => WFST_TRANSLITERATOR.get_or_init(|| {
                Transliterator::from_default_data_with_config(
                    DecoderConfig::default()
                        .with_mode(DecoderMode::Wfst)
                        .with_shadow_log(false),
                )
                .expect("embedded lexicon must load")
            }),
            DecoderMode::Hybrid => HYBRID_TRANSLITERATOR.get_or_init(|| {
                Transliterator::from_default_data_with_config(
                    DecoderConfig::default()
                        .with_mode(DecoderMode::Hybrid)
                        .with_shadow_log(false),
                )
                .expect("embedded lexicon must load")
            }),
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn init_phase_a_legacy_transliterator() -> Transliterator {
    let lexicon = LEXICON_DATA
        .get()
        .expect("phase-A lexicon data must be fetched before engine init");
    Transliterator::from_phase_a_bytes(lexicon, DecoderConfig::default()).expect("phase-A transliterator init failed")
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn init_full_transliterator(config: DecoderConfig) -> Transliterator {
    let lexicon = LEXICON_DATA
        .get()
        .expect("lexicon data must be fetched before full engine init");
    let khpos = KHPOS_DATA
        .get()
        .expect("khpos data must be fetched before full engine init");
    Transliterator::from_compiled_bytes(lexicon, khpos, config).expect("full transliterator init failed")
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn full_engine(mode: DecoderMode) -> &'static Transliterator {
    match mode {
        DecoderMode::Legacy => {
            FULL_LEGACY_TRANSLITERATOR.get_or_init(|| init_full_transliterator(DecoderConfig::default()))
        }
        DecoderMode::Shadow => FULL_SHADOW_TRANSLITERATOR
            .get_or_init(|| init_full_transliterator(DecoderConfig::shadow_interactive().with_shadow_log(false))),
        DecoderMode::Wfst => FULL_WFST_TRANSLITERATOR.get_or_init(|| {
            init_full_transliterator(
                DecoderConfig::default()
                    .with_mode(DecoderMode::Wfst)
                    .with_shadow_log(false),
            )
        }),
        DecoderMode::Hybrid => FULL_HYBRID_TRANSLITERATOR.get_or_init(|| {
            init_full_transliterator(
                DecoderConfig::default()
                    .with_mode(DecoderMode::Hybrid)
                    .with_shadow_log(false),
            )
        }),
    }
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn promote_full_engine() {
    let _ = full_engine(DecoderMode::Legacy);
    FULL_ENGINE_READY.store(true, Ordering::Release);
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StartupProfile {
    Baseline,
    LexiconOnly,
    DeferFull,
    BaselineCompressionAudit,
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
impl StartupProfile {
    fn label(self) -> &'static str {
        match self {
            StartupProfile::Baseline => "baseline",
            StartupProfile::LexiconOnly => "lexicon_only",
            StartupProfile::DeferFull => "defer_full",
            StartupProfile::BaselineCompressionAudit => "baseline_compression_audit",
        }
    }

    fn from_query() -> Self {
        let query = web_sys::window()
            .and_then(|window| window.location().search().ok())
            .unwrap_or_default();
        let mut value = None::<String>;
        for pair in query.trim_start_matches('?').split('&') {
            let Some((key, raw)) = pair.split_once('=') else {
                continue;
            };
            if key == "startup_profile" {
                value = Some(raw.to_ascii_lowercase());
                break;
            }
        }
        match value.as_deref() {
            Some("baseline") => StartupProfile::Baseline,
            Some("lexicon_only") => StartupProfile::LexiconOnly,
            Some("defer_full") => StartupProfile::DeferFull,
            Some("baseline_compression_audit") => StartupProfile::BaselineCompressionAudit,
            _ => StartupProfile::DeferFull,
        }
    }

    fn should_fetch_khpos(self) -> bool {
        !matches!(self, StartupProfile::LexiconOnly)
    }

    fn should_wait_for_full_before_ready(self) -> bool {
        matches!(
            self,
            StartupProfile::Baseline | StartupProfile::BaselineCompressionAudit
        )
    }

    fn should_audit_headers(self) -> bool {
        matches!(self, StartupProfile::BaselineCompressionAudit)
    }
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
#[derive(Clone, Debug)]
struct StartupEvent {
    stage: String,
    t_ms: f64,
    detail: Option<String>,
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
#[derive(Clone, Debug)]
struct StartupTrace {
    profile: StartupProfile,
    started_ms: f64,
    events: Vec<StartupEvent>,
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
impl StartupTrace {
    fn new(profile: StartupProfile) -> Self {
        Self {
            profile,
            started_ms: now_ms(),
            events: Vec::new(),
        }
    }

    fn mark(&mut self, stage: &str) {
        self.events.push(StartupEvent {
            stage: stage.to_owned(),
            t_ms: now_ms() - self.started_ms,
            detail: None,
        });
    }

    fn mark_detail(&mut self, stage: &str, detail: impl Into<String>) {
        self.events.push(StartupEvent {
            stage: stage.to_owned(),
            t_ms: now_ms() - self.started_ms,
            detail: Some(detail.into()),
        });
    }

    fn emit(&self, readiness: EngineReadiness) {
        let mut payload = String::new();
        payload.push('{');
        payload.push_str("\"profile\":\"");
        payload.push_str(self.profile.label());
        payload.push_str("\",\"readiness\":\"");
        payload.push_str(engine_readiness_label(readiness));
        payload.push_str("\",\"events\":[");
        for (index, event) in self.events.iter().enumerate() {
            if index > 0 {
                payload.push(',');
            }
            payload.push('{');
            payload.push_str("\"stage\":\"");
            payload.push_str(&json_escape(&event.stage));
            payload.push_str("\",\"t_ms\":");
            payload.push_str(&format!("{:.2}", event.t_ms));
            if let Some(detail) = &event.detail {
                payload.push_str(",\"detail\":\"");
                payload.push_str(&json_escape(detail));
                payload.push('"');
            }
            payload.push('}');
        }
        payload.push_str("]}");
        web_sys::console::log_1(&payload.into());
    }
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn engine_readiness_label(readiness: EngineReadiness) -> &'static str {
    match readiness {
        EngineReadiness::Booting => "booting",
        EngineReadiness::LegacyReady => "legacy_ready",
        EngineReadiness::FullReady => "full_ready",
        EngineReadiness::Failed => "failed",
    }
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn json_escape(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or(0.0)
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
async fn await_fetch_response(
    promise: js_sys::Promise,
    url: &str,
    response_stage: &str,
    trace: &mut StartupTrace,
) -> Result<web_sys::Response, String> {
    use js_sys::wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let response = JsFuture::from(promise)
        .await
        .map_err(|e| format!("fetch {url} network error: {e:?}"))?;
    let response: web_sys::Response = response
        .dyn_into()
        .map_err(|_| format!("fetch {url}: response cast failed"))?;
    trace.mark(response_stage);
    if response.status() == 304 {
        trace.mark_detail(&format!("{response_stage}.status"), "304_not_modified");
        let cache_busted_url = format!("{url}?_rl_cb={:.0}", now_ms());
        let window = web_sys::window().ok_or_else(|| "no window".to_owned())?;
        let retry = JsFuture::from(window.fetch_with_str(&cache_busted_url))
            .await
            .map_err(|e| format!("fetch {cache_busted_url} network error: {e:?}"))?;
        let retry: web_sys::Response = retry
            .dyn_into()
            .map_err(|_| format!("fetch {cache_busted_url}: response cast failed"))?;
        trace.mark(&format!("{response_stage}.refetch"));
        if !retry.ok() {
            return Err(format!("fetch {cache_busted_url}: HTTP {}", retry.status()));
        }
        return Ok(retry);
    }
    if !response.ok() {
        return Err(format!("fetch {url}: HTTP {}", response.status()));
    }
    Ok(response)
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn capture_response_headers(prefix: &str, response: &web_sys::Response, trace: &mut StartupTrace) {
    let headers = response.headers();
    let encoding = headers
        .get("content-encoding")
        .ok()
        .flatten()
        .unwrap_or_else(|| "none".to_owned());
    trace.mark_detail(&format!("{prefix}.content_encoding"), encoding);
    let cache_control = headers
        .get("cache-control")
        .ok()
        .flatten()
        .unwrap_or_else(|| "missing".to_owned());
    trace.mark_detail(&format!("{prefix}.cache_control"), cache_control);
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
async fn response_to_bytes(
    response: web_sys::Response,
    url: &str,
    stage: &str,
    trace: &mut StartupTrace,
) -> Result<Vec<u8>, String> {
    use wasm_bindgen_futures::JsFuture;

    let buffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|_| format!("fetch {url}: array_buffer() failed"))?,
    )
    .await
    .map_err(|e| format!("fetch {url}: array_buffer await failed: {e:?}"))?;
    trace.mark(stage);
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

fn current_engine_readiness() -> EngineReadiness {
    #[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
    {
        if FULL_ENGINE_READY.load(Ordering::Acquire) {
            EngineReadiness::FullReady
        } else if PHASE_A_LEGACY_TRANSLITERATOR.get().is_some() {
            EngineReadiness::LegacyReady
        } else {
            EngineReadiness::Booting
        }
    }
    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    {
        if LEGACY_TRANSLITERATOR.get().is_some() {
            EngineReadiness::FullReady
        } else {
            EngineReadiness::Booting
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn data_asset_url(filename: &str) -> String {
    let base = option_env!("KHMERIME_BASE_PATH")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let trimmed = value.trim_matches('/');
            if trimmed.is_empty() {
                "/".to_owned()
            } else {
                format!("/{trimmed}")
            }
        });

    match base {
        Some(root) if root == "/" => format!("/assets/data/{filename}"),
        Some(root) => format!("{root}/assets/data/{filename}"),
        None => format!("./assets/data/{filename}"),
    }
}

#[component]
fn App() -> Element {
    let initial_readiness = current_engine_readiness();
    let mut engine_readiness = use_signal(|| initial_readiness);
    let mut engine_ready = use_signal(|| initial_readiness.is_ready());
    let mut engine_progress = use_signal(|| if initial_readiness.is_ready() { 100u8 } else { 0u8 });
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
        spawn(async move {
            if engine_readiness() == EngineReadiness::FullReady {
                engine_progress.set(100);
                engine_ready.set(true);
                return;
            }

            engine_readiness.set(EngineReadiness::Booting);
            engine_ready.set(false);
            engine_progress.set(10);
            #[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
            {
                let profile = StartupProfile::from_query();
                let mut trace = StartupTrace::new(profile);
                trace.mark("bootstrap.ready");

                let Some(window) = web_sys::window() else {
                    let message = "startup failed: no window".to_owned();
                    web_sys::console::error_1(&message.clone().into());
                    trace.mark_detail("startup.error", message);
                    engine_progress.set(100);
                    engine_readiness.set(EngineReadiness::Failed);
                    engine_ready.set(true);
                    trace.emit(EngineReadiness::Failed);
                    return;
                };

                let lexicon_url = data_asset_url("roman_lookup.lexicon.bin");
                let khpos_url = data_asset_url("khpos.stats.bin");

                trace.mark("fetch.lexicon.start");
                let lexicon_promise = window.fetch_with_str(&lexicon_url);
                let khpos_promise = if profile.should_fetch_khpos() {
                    trace.mark("fetch.khpos.start");
                    Some(window.fetch_with_str(&khpos_url))
                } else {
                    None
                };

                let lexicon_response =
                    match await_fetch_response(lexicon_promise, &lexicon_url, "fetch.lexicon.end", &mut trace).await {
                        Ok(response) => response,
                        Err(error) => {
                            web_sys::console::error_1(&error.clone().into());
                            trace.mark_detail("startup.error", error);
                            engine_progress.set(100);
                            engine_readiness.set(EngineReadiness::Failed);
                            engine_ready.set(true);
                            trace.emit(EngineReadiness::Failed);
                            return;
                        }
                    };
                if profile.should_audit_headers() {
                    capture_response_headers("lexicon", &lexicon_response, &mut trace);
                }
                let lexicon =
                    match response_to_bytes(lexicon_response, &lexicon_url, "array_buffer.lexicon.end", &mut trace)
                        .await
                    {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            web_sys::console::error_1(&error.clone().into());
                            trace.mark_detail("startup.error", error);
                            engine_progress.set(100);
                            engine_readiness.set(EngineReadiness::Failed);
                            engine_ready.set(true);
                            trace.emit(EngineReadiness::Failed);
                            return;
                        }
                    };
                trace.mark("Transliterator::from_phase_a_bytes.start");
                let phase_a_transliterator =
                    match Transliterator::from_phase_a_bytes(&lexicon, DecoderConfig::default()) {
                        Ok(transliterator) => transliterator,
                        Err(error) => {
                            let message = format!("phase-A transliterator init failed: {error}");
                            web_sys::console::error_1(&message.clone().into());
                            trace.mark_detail("startup.error", message);
                            engine_progress.set(100);
                            engine_readiness.set(EngineReadiness::Failed);
                            engine_ready.set(true);
                            trace.emit(EngineReadiness::Failed);
                            return;
                        }
                    };
                trace.mark("Transliterator::from_phase_a_bytes.end");
                let _ = PHASE_A_LEGACY_TRANSLITERATOR.set(phase_a_transliterator);
                let _ = LEXICON_DATA.set(lexicon);

                if profile.should_wait_for_full_before_ready() {
                    let Some(promise) = khpos_promise else {
                        let message = "startup failed: profile expected khpos fetch".to_owned();
                        web_sys::console::error_1(&message.clone().into());
                        trace.mark_detail("startup.error", message);
                        engine_progress.set(100);
                        engine_readiness.set(EngineReadiness::Failed);
                        engine_ready.set(true);
                        trace.emit(EngineReadiness::Failed);
                        return;
                    };
                    let khpos_response =
                        match await_fetch_response(promise, &khpos_url, "fetch.khpos.end", &mut trace).await {
                            Ok(response) => response,
                            Err(error) => {
                                web_sys::console::error_1(&error.clone().into());
                                trace.mark_detail("startup.error", error);
                                engine_progress.set(100);
                                engine_readiness.set(EngineReadiness::Failed);
                                engine_ready.set(true);
                                trace.emit(EngineReadiness::Failed);
                                return;
                            }
                        };
                    if profile.should_audit_headers() {
                        capture_response_headers("khpos", &khpos_response, &mut trace);
                    }
                    let khpos =
                        match response_to_bytes(khpos_response, &khpos_url, "array_buffer.khpos.end", &mut trace).await
                        {
                            Ok(bytes) => bytes,
                            Err(error) => {
                                web_sys::console::error_1(&error.clone().into());
                                trace.mark_detail("startup.error", error);
                                engine_progress.set(100);
                                engine_readiness.set(EngineReadiness::Failed);
                                engine_ready.set(true);
                                trace.emit(EngineReadiness::Failed);
                                return;
                            }
                        };
                    let _ = KHPOS_DATA.set(khpos);
                    engine_progress.set(82);
                    trace.mark("Transliterator::from_compiled_bytes.start");
                    promote_full_engine();
                    trace.mark("Transliterator::from_compiled_bytes.end");
                    engine_progress.set(100);
                    engine_readiness.set(EngineReadiness::FullReady);
                    engine_ready.set(true);
                    trace.mark("phase.full_ready");
                    trace.emit(EngineReadiness::FullReady);
                    return;
                }

                engine_progress.set(70);
                engine_readiness.set(EngineReadiness::LegacyReady);
                engine_ready.set(true);
                trace.mark("phase.legacy_ready");
                if !profile.should_fetch_khpos() {
                    engine_progress.set(100);
                    trace.emit(EngineReadiness::LegacyReady);
                    return;
                }

                engine_progress.set(80);
                let Some(promise) = khpos_promise else {
                    let message = "startup failed: khpos promise missing".to_owned();
                    web_sys::console::error_1(&message.clone().into());
                    trace.mark_detail("startup.error", message);
                    engine_progress.set(100);
                    engine_readiness.set(EngineReadiness::Failed);
                    engine_ready.set(true);
                    trace.emit(EngineReadiness::Failed);
                    return;
                };
                let khpos_response =
                    match await_fetch_response(promise, &khpos_url, "fetch.khpos.end", &mut trace).await {
                        Ok(response) => response,
                        Err(error) => {
                            web_sys::console::error_1(&error.clone().into());
                            trace.mark_detail("startup.error", error);
                            engine_progress.set(100);
                            engine_readiness.set(EngineReadiness::Failed);
                            engine_ready.set(true);
                            trace.emit(EngineReadiness::Failed);
                            return;
                        }
                    };
                let khpos =
                    match response_to_bytes(khpos_response, &khpos_url, "array_buffer.khpos.end", &mut trace).await {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            web_sys::console::error_1(&error.clone().into());
                            trace.mark_detail("startup.error", error);
                            engine_progress.set(100);
                            engine_readiness.set(EngineReadiness::Failed);
                            engine_ready.set(true);
                            trace.emit(EngineReadiness::Failed);
                            return;
                        }
                    };
                let _ = KHPOS_DATA.set(khpos);
                trace.mark("Transliterator::from_compiled_bytes.start");
                promote_full_engine();
                trace.mark("Transliterator::from_compiled_bytes.end");
                engine_progress.set(100);
                engine_readiness.set(EngineReadiness::FullReady);
                engine_ready.set(true);
                trace.mark("phase.full_ready");
                trace.emit(EngineReadiness::FullReady);
                return;
            }

            #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
            {
                engine_progress.set(85);
                let history = HashMap::new();
                let _ = engine(DecoderMode::Legacy).suggest("a", &history);
                engine_progress.set(100);
                engine_readiness.set(EngineReadiness::FullReady);
                engine_ready.set(true);
            }
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
