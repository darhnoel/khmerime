use dioxus::prelude::*;
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
use roman_lookup::{DecoderConfig, Transliterator};

#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
use crate::engine_registry::engine;
use crate::engine_registry::EngineReadiness;
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
use crate::engine_registry::{promote_full_engine, store_full_support_data, store_phase_a_assets};
use crate::startup_signals::StartupSignals;
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
use roman_lookup::DecoderMode;
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
use std::collections::HashMap;

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

pub(crate) fn start_engine_bootstrap(signals: StartupSignals) {
    let mut engine_readiness = signals.engine_readiness;
    let mut engine_ready = signals.engine_ready;
    let mut engine_progress = signals.engine_progress;

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
            let next_word_url = data_asset_url("next_word.stats.bin");

            trace.mark("fetch.lexicon.start");
            let lexicon_promise = window.fetch_with_str(&lexicon_url);
            let mut khpos_promise = if profile.should_fetch_khpos() {
                trace.mark("fetch.khpos.start");
                Some(window.fetch_with_str(&khpos_url))
            } else {
                None
            };
            let mut next_word_promise = if profile.should_fetch_khpos() {
                trace.mark("fetch.next_word.start");
                Some(window.fetch_with_str(&next_word_url))
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
                match response_to_bytes(lexicon_response, &lexicon_url, "array_buffer.lexicon.end", &mut trace).await {
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
            let phase_a_transliterator = match Transliterator::from_phase_a_bytes(&lexicon, DecoderConfig::default()) {
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
            store_phase_a_assets(lexicon, phase_a_transliterator);

            if profile.should_wait_for_full_before_ready() {
                let Some(khpos_fetch) = khpos_promise.take() else {
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
                    match await_fetch_response(khpos_fetch, &khpos_url, "fetch.khpos.end", &mut trace).await {
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
                let Some(next_word_fetch) = next_word_promise.take() else {
                    let message = "startup failed: profile expected next-word fetch".to_owned();
                    web_sys::console::error_1(&message.clone().into());
                    trace.mark_detail("startup.error", message);
                    engine_progress.set(100);
                    engine_readiness.set(EngineReadiness::Failed);
                    engine_ready.set(true);
                    trace.emit(EngineReadiness::Failed);
                    return;
                };
                let next_word_response = match await_fetch_response(
                    next_word_fetch,
                    &next_word_url,
                    "fetch.next_word.end",
                    &mut trace,
                )
                .await
                {
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
                    capture_response_headers("next_word", &next_word_response, &mut trace);
                }
                let next_word = match response_to_bytes(
                    next_word_response,
                    &next_word_url,
                    "array_buffer.next_word.end",
                    &mut trace,
                )
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
                store_full_support_data(khpos, next_word);
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
            let Some(khpos_fetch) = khpos_promise.take() else {
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
                match await_fetch_response(khpos_fetch, &khpos_url, "fetch.khpos.end", &mut trace).await {
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
            let Some(next_word_fetch) = next_word_promise.take() else {
                let message = "startup failed: next-word promise missing".to_owned();
                web_sys::console::error_1(&message.clone().into());
                trace.mark_detail("startup.error", message);
                engine_progress.set(100);
                engine_readiness.set(EngineReadiness::Failed);
                engine_ready.set(true);
                trace.emit(EngineReadiness::Failed);
                return;
            };
            let next_word_response =
                match await_fetch_response(next_word_fetch, &next_word_url, "fetch.next_word.end", &mut trace).await {
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
            let khpos = match response_to_bytes(khpos_response, &khpos_url, "array_buffer.khpos.end", &mut trace).await
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
            let next_word = match response_to_bytes(
                next_word_response,
                &next_word_url,
                "array_buffer.next_word.end",
                &mut trace,
            )
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
            store_full_support_data(khpos, next_word);
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
}
