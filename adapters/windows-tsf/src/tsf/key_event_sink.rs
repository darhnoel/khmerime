//! `ITfKeyEventSink` implementation.

use std::sync::{Arc, Mutex};

use windows::core::{implement, Error, Result, GUID};
use windows::Win32::Foundation::{BOOL, E_FAIL, FALSE, LPARAM, TRUE, WPARAM};
use windows::Win32::UI::TextServices::{ITfContext, ITfKeyEventSink, ITfKeyEventSink_Impl};

use crate::com::text_service::TextServiceState;
use crate::diagnostics::log;
use crate::input::key_convert::{convert_windows_key, would_handle_windows_key, ConvertedKey, WindowsKeyInput};
use crate::session_driver::WindowsSessionDriver;
use crate::tsf::edit_session::request_render_edit_session;
use crate::WindowsTsfCallback;

/// Key sink callbacks expected in the first runnable TSF spike.
pub const KEY_EVENT_SINK_CALLBACKS: &[&str] = &[
    "OnSetFocus",
    "OnTestKeyDown",
    "OnKeyDown",
    "OnTestKeyUp",
    "OnKeyUp",
    "OnPreservedKey",
];

#[implement(ITfKeyEventSink)]
pub struct KhmerImeKeyEventSink {
    state: Arc<Mutex<TextServiceState>>,
}

impl KhmerImeKeyEventSink {
    pub fn new(state: Arc<Mutex<TextServiceState>>) -> Self {
        Self { state }
    }
}

impl ITfKeyEventSink_Impl for KhmerImeKeyEventSink_Impl {
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        log(format!(
            "KeyEventSink::OnSetFocus foreground={} noop",
            fforeground.as_bool()
        ));
        Ok(())
    }

    fn OnTestKeyDown(&self, _pic: Option<&ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let input = windows_key_input(wparam, lparam);
        let handled = would_handle_windows_key(input);
        log(format!(
            "KeyEventSink::OnTestKeyDown vk=0x{:X} char={:?} handled={handled}",
            input.virtual_key, input.translated_char
        ));
        Ok(bool_to_win32(handled))
    }

    fn OnTestKeyUp(&self, _pic: Option<&ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnKeyDown(&self, pic: Option<&ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let ConvertedKey::Event(event) = convert_windows_key(windows_key_input(wparam, lparam)) else {
            log(format!("KeyEventSink::OnKeyDown passthrough vk=0x{:X}", wparam.0));
            return Ok(FALSE);
        };

        let (client_id, render_state) = {
            let mut state = lock_state(&self.state)?;
            if state.driver.is_none() {
                state.driver = match WindowsSessionDriver::from_default_data() {
                    Ok(mut driver) => {
                        driver.process_callback(WindowsTsfCallback::Activate);
                        log("KeyEventSink::OnKeyDown lazy driver initialized");
                        Some(driver)
                    }
                    Err(err) => {
                        log(format!("KeyEventSink::OnKeyDown driver init failed: {err}"));
                        return Ok(FALSE);
                    }
                };
            }
            let Some(driver) = state.driver.as_mut() else {
                log("KeyEventSink::OnKeyDown driver unavailable after init");
                return Ok(FALSE);
            };
            let render_state = driver.process_callback(WindowsTsfCallback::KeyDown(event));
            (state.client_id, render_state)
        };
        log(format!(
            "KeyEventSink::OnKeyDown keyval=0x{:X} consumed={} commit={:?} preedit_len={} candidates={}",
            event.keyval,
            render_state.consumed,
            render_state.commit_text,
            render_state.preedit.len(),
            render_state.candidates.len()
        ));

        if let Some(context) = pic {
            if let Err(e) =
                request_render_edit_session(context, client_id, render_state.clone(), Arc::clone(&self.state))
            {
                // Never propagate edit-session failures to TSF. TSF does not expect OnKeyDown to
                // return a failure HRESULT and may deactivate the text service if it does.
                log(format!("KeyEventSink::OnKeyDown edit session failed: {e:?}"));
            }
        }

        Ok(bool_to_win32(render_state.consumed))
    }

    fn OnKeyUp(&self, _pic: Option<&ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnPreservedKey(&self, _pic: Option<&ITfContext>, _rguid: *const GUID) -> Result<BOOL> {
        Ok(FALSE)
    }
}

fn lock_state(state: &Arc<Mutex<TextServiceState>>) -> Result<std::sync::MutexGuard<'_, TextServiceState>> {
    state.lock().map_err(|_| Error::from(E_FAIL))
}

fn bool_to_win32(value: bool) -> BOOL {
    if value {
        TRUE
    } else {
        FALSE
    }
}

fn windows_key_input(wparam: WPARAM, lparam: LPARAM) -> WindowsKeyInput {
    let virtual_key = wparam.0 as u32;
    WindowsKeyInput {
        virtual_key,
        scan_code: ((lparam.0 as u64 >> 16) & 0xff) as u32,
        state: 0,
        translated_char: translated_ascii_char(virtual_key),
    }
}

fn translated_ascii_char(virtual_key: u32) -> Option<char> {
    match virtual_key {
        0x30..=0x39 => char::from_u32(virtual_key),
        0x41..=0x5A => char::from_u32(virtual_key).map(|ch| ch.to_ascii_lowercase()),
        0xBA..=0xC0 | 0xDB..=0xDE => None,
        _ => None,
    }
}
