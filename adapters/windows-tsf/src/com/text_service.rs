//! `ITfTextInputProcessor` shell for the KhmerIME TSF service.

use std::sync::{mpsc::Receiver, Arc, Mutex};

use windows::core::{implement, Error, Interface, Result};
use windows::Win32::Foundation::{E_FAIL, TRUE};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfKeyEventSink, ITfKeystrokeMgr, ITfTextInputProcessor, ITfTextInputProcessor_Impl, ITfThreadMgr,
};

use crate::com::dll_module;
use crate::diagnostics::log;
use crate::session_driver::WindowsSessionDriver;
use crate::tsf::candidates::CandidateWindow;
use crate::tsf::key_event_sink::KhmerImeKeyEventSink;

/// Planned lifecycle callbacks for the TSF text service shell.
pub const TEXT_SERVICE_LIFECYCLE: &[&str] = &["Activate", "Deactivate"];

pub struct TextServiceState {
    pub driver: Option<WindowsSessionDriver>,
    pub pending_driver: Option<Receiver<std::result::Result<WindowsSessionDriver, String>>>,
    pub thread_mgr: Option<ITfThreadMgr>,
    pub client_id: u32,
    pub key_sink: Option<ITfKeyEventSink>,
    pub composition: Option<ITfComposition>,
    /// Native Win32 popup window for the candidate list.
    /// Created lazily on first use; hidden on Deactivate.
    pub candidate_window: Option<CandidateWindow>,
}

impl Default for TextServiceState {
    fn default() -> Self {
        Self {
            driver: None,
            pending_driver: None,
            thread_mgr: None,
            client_id: 0,
            key_sink: None,
            composition: None,
            candidate_window: None,
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
            state.driver = None;
            state.pending_driver = Some(crate::session_driver::spawn_default_driver_warmup());
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
            if let Some(driver) = state.driver.as_mut() {
                driver.process_callback(crate::WindowsTsfCallback::Deactivate);
            }
            let thread_mgr = state.thread_mgr.take();
            let client_id = state.client_id;
            let key_sink = state.key_sink.take();
            state.composition = None;
            state.client_id = 0;
            state.driver = None;
            state.pending_driver = None;
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
