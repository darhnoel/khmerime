//! `ITfKeyEventSink` implementation.

use std::sync::{mpsc::TryRecvError, Arc, Mutex};

use windows::core::{implement, Error, Result, GUID};
use windows::Win32::Foundation::{BOOL, E_FAIL, FALSE, LPARAM, TRUE, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::UI::TextServices::{ITfContext, ITfKeyEventSink, ITfKeyEventSink_Impl};

use crate::com::text_service::TextServiceState;
use crate::diagnostics::log;
use crate::input::key_convert::{
    convert_windows_key, would_handle_windows_key, ConvertedKey, WindowsKeyInput, STATE_ALT_MASK, STATE_CONTROL_MASK,
    STATE_RELEASE_MASK, STATE_SUPER_MASK,
};
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
        let handled = would_handle_windows_key(input) && driver_can_handle_key(&self.state, input)?;
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
            poll_pending_driver(&mut state);
            let Some(driver) = state.driver.as_mut() else {
                log("KeyEventSink::OnKeyDown driver still warming; passthrough");
                return Ok(FALSE);
            };
            if is_idle_digit_passthrough(driver, event.keyval) {
                log(format!(
                    "KeyEventSink::OnKeyDown idle digit passthrough keyval=0x{:X}",
                    event.keyval
                ));
                return Ok(FALSE);
            }
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

fn driver_can_handle_key(state: &Arc<Mutex<TextServiceState>>, input: WindowsKeyInput) -> Result<bool> {
    let mut state = lock_state(state)?;
    poll_pending_driver(&mut state);
    let Some(driver) = state.driver.as_ref() else {
        return Ok(false);
    };
    let ConvertedKey::Event(event) = convert_windows_key(input) else {
        return Ok(false);
    };
    Ok(!is_idle_digit_passthrough(driver, event.keyval))
}

fn poll_pending_driver(state: &mut TextServiceState) {
    let Some(receiver) = state.pending_driver.take() else {
        return;
    };

    match receiver.try_recv() {
        Ok(Ok(driver)) => {
            state.driver = Some(driver);
            log("KeyEventSink::driver warmup completed");
        }
        Ok(Err(err)) => {
            log(format!("KeyEventSink::driver warmup failed: {err}"));
        }
        Err(TryRecvError::Empty) => {
            state.pending_driver = Some(receiver);
        }
        Err(TryRecvError::Disconnected) => {
            log("KeyEventSink::driver warmup disconnected");
        }
    }
}

fn bool_to_win32(value: bool) -> BOOL {
    if value {
        TRUE
    } else {
        FALSE
    }
}

fn is_idle_digit_passthrough(driver: &crate::session_driver::WindowsSessionDriver, keyval: u32) -> bool {
    let Some(ch) = char::from_u32(keyval) else {
        return false;
    };
    if !ch.is_ascii_digit() {
        return false;
    }

    let snapshot = driver.session().snapshot();
    snapshot.preedit.is_empty() && snapshot.candidates.is_empty()
}

fn windows_key_input(wparam: WPARAM, lparam: LPARAM) -> WindowsKeyInput {
    windows_key_input_with_state(wparam, lparam, current_modifier_state(lparam))
}

fn windows_key_input_with_state(wparam: WPARAM, lparam: LPARAM, state: u32) -> WindowsKeyInput {
    let virtual_key = wparam.0 as u32;
    WindowsKeyInput {
        virtual_key,
        scan_code: ((lparam.0 as u64 >> 16) & 0xff) as u32,
        state,
        translated_char: translated_ascii_char(virtual_key),
    }
}

fn current_modifier_state(lparam: LPARAM) -> u32 {
    let mut state = 0;
    if key_is_down(0x11) {
        state |= STATE_CONTROL_MASK;
    }
    if key_is_down(0x12) {
        state |= STATE_ALT_MASK;
    }
    if key_is_down(0x5B) || key_is_down(0x5C) {
        state |= STATE_SUPER_MASK;
    }
    if ((lparam.0 as u64 >> 31) & 1) != 0 {
        state |= STATE_RELEASE_MASK;
    }
    state
}

fn key_is_down(virtual_key: i32) -> bool {
    unsafe { (GetKeyState(virtual_key) as u16 & 0x8000) != 0 }
}

fn translated_ascii_char(virtual_key: u32) -> Option<char> {
    match virtual_key {
        0x30..=0x39 => char::from_u32(virtual_key),
        0x41..=0x5A => char::from_u32(virtual_key).map(|ch| ch.to_ascii_lowercase()),
        0xBA..=0xC0 | 0xDB..=0xDE => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use khmerime_core::{DecoderConfig, Transliterator};
    use khmerime_session::ImeSession;

    use crate::input::key_convert::{ConvertedKey, STATE_CONTROL_MASK};
    use crate::session_driver::WindowsSessionDriver;

    #[test]
    fn ctrl_a_passes_through_to_host_shortcut() {
        let input = windows_key_input_with_state(WPARAM(0x41), LPARAM(0), STATE_CONTROL_MASK);

        assert_eq!(convert_windows_key(input), ConvertedKey::PassThrough);
        assert!(!would_handle_windows_key(input));
    }

    #[test]
    fn plain_a_remains_ime_handled() {
        let input = windows_key_input_with_state(WPARAM(0x41), LPARAM(0), 0);

        assert!(matches!(convert_windows_key(input), ConvertedKey::Event(_)));
        assert!(would_handle_windows_key(input));
    }

    #[test]
    fn idle_digit_passes_through_before_composition() {
        let driver = test_driver();

        assert!(is_idle_digit_passthrough(&driver, '2' as u32));
    }

    #[test]
    fn digit_still_handles_candidate_selection_when_composing() {
        let mut driver = test_driver();
        driver.process_key_event(k('f'));
        driver.process_key_event(k('o'));
        driver.process_key_event(k('o'));

        assert!(!is_idle_digit_passthrough(&driver, '2' as u32));
    }

    fn test_driver() -> WindowsSessionDriver {
        let fixture = "foo\tfirst\nfoo\tsecond\n";
        let transliterator = Transliterator::from_tsv_str_with_config(fixture, DecoderConfig::shadow_interactive())
            .expect("fixture must parse");
        WindowsSessionDriver::new(ImeSession::new(transliterator, HashMap::new()))
    }

    fn k(ch: char) -> khmerime_session::NativeKeyEvent {
        khmerime_session::NativeKeyEvent {
            keyval: ch as u32,
            keycode: ch as u32,
            state: 0,
        }
    }
}
