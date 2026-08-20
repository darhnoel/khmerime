//! `ITfTextInputProcessor` shell for the KhmerIME TSF service.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use windows::core::{implement, Error, Interface, Result};
use windows::Win32::Foundation::{E_FAIL, TRUE};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfKeyEventSink, ITfKeystrokeMgr, ITfTextInputProcessor, ITfTextInputProcessor_Impl, ITfThreadMgr,
};

use khmerime_session::CursorLocation;

use crate::com::dll_module;
use crate::diagnostics::log;
use crate::session_driver::WindowsSessionDriver;
use crate::tsf::candidates::CandidateWindow;
use crate::tsf::key_event_sink::KhmerImeKeyEventSink;

/// Planned lifecycle callbacks for the TSF text service shell.
pub const TEXT_SERVICE_LIFECYCLE: &[&str] = &["Activate", "Deactivate"];

pub struct TextServiceState {
    /// The portable half: session, debounce timestamps, popup handle. Separated because the rest
    /// of this struct holds COM interfaces and an HWND and can never leave this thread.
    pub shared: std::sync::Arc<crate::tsf::refine::SessionShared>,
    pub thread_mgr: Option<ITfThreadMgr>,
    pub client_id: u32,
    pub key_sink: Option<ITfKeyEventSink>,
    pub composition: Option<ITfComposition>,
    /// Native Win32 popup window for the candidate list.
    /// Created lazily on first use; hidden on Deactivate.
    pub candidate_window: Option<CandidateWindow>,
    /// Last preedit text successfully written to the TSF composition range.
    /// Used to skip redundant edit sessions when only candidates changed.
    pub current_preedit: String,
    /// Last candidate popup anchor resolved from a composition range.
    /// Reused for candidate-only updates that don't need a TSF edit session.
    pub last_candidate_anchor: Option<CursorLocation>,
    /// When the current warmup began, for measuring the leak window.
    ///
    /// Keys arriving before the driver lands are passed through to the host as
    /// raw roman today (see ADR-0001). These two fields record how wide that
    /// window is and how many keys actually fall into it, per host process.
    pub warmup_started_at: Option<Instant>,
    /// Keys passed through to the host because the driver was still warming.
    pub warmup_passthrough_count: u32,
    /// Stop flag for the refine worker; cleared on Deactivate so it does not outlive the profile.
    pub refine_worker: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl Default for TextServiceState {
    fn default() -> Self {
        Self {
            shared: Default::default(),
            thread_mgr: None,
            client_id: 0,
            key_sink: None,
            composition: None,
            candidate_window: None,
            current_preedit: String::new(),
            last_candidate_anchor: None,
            warmup_started_at: None,
            warmup_passthrough_count: 0,
            refine_worker: None,
        }
    }
}

#[implement(ITfTextInputProcessor)]
pub struct KhmerImeTextService {
    state: Arc<Mutex<TextServiceState>>,
}

impl KhmerImeTextService {
    pub fn new() -> Self {
        dll_module::object_created();
        Self {
            state: Arc::new(Mutex::new(TextServiceState::default())),
        }
    }
}

impl Drop for KhmerImeTextService {
    fn drop(&mut self) {
        dll_module::object_released();
    }
}

impl ITfTextInputProcessor_Impl for KhmerImeTextService_Impl {
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        log(format!("TextService::Activate tid={tid}"));
        let (old_thread_mgr, old_client_id, _old_key_sink) = {
            let mut state = lock_state(&self.state)?;
            let old_thread_mgr = state.thread_mgr.take();
            let old_client_id = state.client_id;
            let old_key_sink = state.key_sink.take();
            (old_thread_mgr, old_client_id, old_key_sink)
        };
        if old_client_id != 0 {
            if let Some(thread_mgr) = &old_thread_mgr {
                if let Ok(keystroke_mgr) = thread_mgr.cast::<ITfKeystrokeMgr>() {
                    unsafe {
                        let _ = keystroke_mgr.UnadviseKeyEventSink(old_client_id);
                    }
                    log(format!(
                        "TextService::Activate unadvised stale key sink tid={old_client_id}"
                    ));
                }
            }
        }

        {
            let mut state = lock_state(&self.state)?;
            // Build Phase A on this thread so the driver exists before the first
            // keystroke can arrive. Phase A skips the expensive ranked-lexicon and
            // search-index stages, so this is a short block, not the full engine
            // build. The full engine swaps in from a background thread. See ADR-0001.
            let built = match crate::session_driver::WindowsSessionDriver::from_phase_a_data_traced() {
                Ok(driver) => Some(activate_driver(driver)),
                Err(error) => {
                    log(format!("TextService::Activate phase A build failed: {error}"));
                    None
                }
            };
            if let Ok(mut slot) = state.shared.driver.lock() {
                *slot = built;
            }
            crate::tsf::refine::bind_thread_state(Arc::clone(&self.state));
            state.refine_worker = Some(crate::tsf::refine::spawn_refine_worker(Arc::clone(&state.shared)));
            state.warmup_started_at = Some(Instant::now());
            state.warmup_passthrough_count = 0;
            state.composition = None;
            state.client_id = tid;
            state.thread_mgr = ptim.cloned();
        }

        if let Some(thread_mgr) = ptim {
            if let Ok(keystroke_mgr) = thread_mgr.cast::<ITfKeystrokeMgr>() {
                let sink: ITfKeyEventSink = KhmerImeKeyEventSink::new(Arc::clone(&self.state)).into();
                unsafe {
                    keystroke_mgr.AdviseKeyEventSink(tid, &sink, TRUE)?;
                }
                log("TextService::Activate key sink advised");
                lock_state(&self.state)?.key_sink = Some(sink);
            }
        }

        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        log("TextService::Deactivate");
        let (thread_mgr, client_id, _key_sink) = {
            let mut state = lock_state(&self.state)?;
            if let Ok(mut slot) = state.shared.driver.lock() {
                if let Some(driver) = slot.as_mut() {
                    driver.process_callback(crate::WindowsTsfCallback::Deactivate);
                }
                *slot = None;
            }
            if let Some(running) = state.refine_worker.take() {
                running.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            crate::tsf::refine::clear_thread_state();
            let thread_mgr = state.thread_mgr.take();
            let client_id = state.client_id;
            let key_sink = state.key_sink.take();
            state.composition = None;
            state.client_id = 0;
            state.current_preedit.clear();
            (thread_mgr, client_id, key_sink)
        };

        if let Some(thread_mgr) = &thread_mgr {
            if let Ok(keystroke_mgr) = thread_mgr.cast::<ITfKeystrokeMgr>() {
                unsafe {
                    let _ = keystroke_mgr.UnadviseKeyEventSink(client_id);
                }
            }
        }

        if let Ok(state) = lock_state(&self.state) {
            if let Some(w) = &state.candidate_window {
                w.hide();
            }
        }
        Ok(())
    }
}

fn lock_state(state: &Arc<Mutex<TextServiceState>>) -> Result<std::sync::MutexGuard<'_, TextServiceState>> {
    state.lock().map_err(|_| Error::from(E_FAIL))
}

fn activate_driver(mut driver: WindowsSessionDriver) -> WindowsSessionDriver {
    driver.process_callback(crate::WindowsTsfCallback::Activate);
    driver
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use khmerime_core::{DecoderConfig, Transliterator};
    use khmerime_session::ImeSession;

    use super::*;

    #[test]
    fn activating_text_service_activates_phase_a_session() {
        let transliterator =
            Transliterator::from_tsv_str_with_config("jea\tcandidate\n", DecoderConfig::shadow_interactive())
                .expect("fixture must parse");
        let driver = WindowsSessionDriver::new(ImeSession::new(transliterator, HashMap::new()));

        let driver = activate_driver(driver);
        let snapshot = driver.session().snapshot();

        assert!(snapshot.enabled);
        assert!(snapshot.focused);
    }
}
