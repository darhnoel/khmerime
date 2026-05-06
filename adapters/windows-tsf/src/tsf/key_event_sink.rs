//! `ITfKeyEventSink` implementation.

use std::sync::{mpsc::TryRecvError, Arc, Mutex};

use windows::core::{implement, Error, Result, GUID};
use windows::Win32::Foundation::{BOOL, E_FAIL, FALSE, LPARAM, TRUE, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::UI::TextServices::{ITfContext, ITfKeyEventSink, ITfKeyEventSink_Impl};

use khmerime_session::{NativeKeyEvent, SessionSnapshot};

use crate::com::text_service::TextServiceState;
use crate::diagnostics::log;
use crate::input::key_convert::{
    convert_windows_key, ConvertedKey, WindowsKeyInput, SESSION_KEY_BACKSPACE, SESSION_KEY_DOWN, SESSION_KEY_ESCAPE,
    SESSION_KEY_LEFT, SESSION_KEY_RETURN, SESSION_KEY_RIGHT, SESSION_KEY_SPACE, SESSION_KEY_UP, STATE_ALT_MASK,
    STATE_CONTROL_MASK, STATE_RELEASE_MASK, STATE_SUPER_MASK,
};

const VK_SHIFT: i32 = 0x10;
use crate::tsf::edit_session::{refresh_candidates, request_render_edit_session};
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
        let handled = driver_would_handle_key(&self.state, input)?;
        Ok(bool_to_win32(handled))
    }

    fn OnTestKeyUp(&self, _pic: Option<&ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnKeyDown(&self, pic: Option<&ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let ConvertedKey::Event(event) = convert_windows_key(windows_key_input(wparam, lparam)) else {
            return Ok(FALSE);
        };

        let (client_id, current_preedit, has_active_composition, render_state) = {
            let mut state = lock_state(&self.state)?;
            poll_pending_driver(&mut state);
            let Some(driver) = state.driver.as_mut() else {
                log("KeyEventSink::OnKeyDown driver still warming; passthrough");
                return Ok(FALSE);
            };
            if !event_would_be_consumed(event, &driver.session().snapshot()) {
                return Ok(FALSE);
            }
            let render_state = driver.process_callback(WindowsTsfCallback::KeyDown(event));
            let current_preedit = state.current_preedit.clone();
            let has_active_composition = state.composition.is_some();
            (state.client_id, current_preedit, has_active_composition, render_state)
        };

        if let Some(context) = pic {
            // Only request a TSF edit session when composition text actually changes.
            // Candidate-only updates (Space / arrow cycling) skip the TSF document
            // mutation and refresh the popup window directly with the cached anchor.
            let preedit_changed = render_state.commit_text.is_some()
                || render_state.preedit != current_preedit
                || (render_state.preedit.is_empty() && has_active_composition);

            if preedit_changed {
                if let Err(e) =
                    request_render_edit_session(context, client_id, render_state.clone(), Arc::clone(&self.state))
                {
                    // Never propagate edit-session failures to TSF.
                    log(format!("KeyEventSink::OnKeyDown edit session failed: {e:?}"));
                }
            } else {
                refresh_candidates(&self.state, &render_state);
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

fn driver_would_handle_key(state: &Arc<Mutex<TextServiceState>>, input: WindowsKeyInput) -> Result<bool> {
    let mut state = lock_state(state)?;
    poll_pending_driver(&mut state);
    let Some(driver) = state.driver.as_ref() else {
        return Ok(false);
    };
    let ConvertedKey::Event(event) = convert_windows_key(input) else {
        return Ok(false);
    };
    Ok(event_would_be_consumed(event, &driver.session().snapshot()))
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

fn windows_key_input(wparam: WPARAM, lparam: LPARAM) -> WindowsKeyInput {
    windows_key_input_with_state(wparam, lparam, current_modifier_state(lparam))
}

fn windows_key_input_with_state(wparam: WPARAM, lparam: LPARAM, state: u32) -> WindowsKeyInput {
    let virtual_key = wparam.0 as u32;
    WindowsKeyInput {
        virtual_key,
        scan_code: ((lparam.0 as u64 >> 16) & 0xff) as u32,
        state,
        translated_char: translated_key_char(virtual_key),
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

fn shift_is_down() -> bool {
    key_is_down(VK_SHIFT)
}

fn key_is_down(virtual_key: i32) -> bool {
    unsafe { (GetKeyState(virtual_key) as u16 & 0x8000) != 0 }
}

fn translated_key_char(virtual_key: u32) -> Option<char> {
    let shift = shift_is_down();
    match virtual_key {
        0x30..=0x39 => {
            if shift {
                Some(shift_digit_char(virtual_key))
            } else {
                char::from_u32(virtual_key)
            }
        }
        0x41..=0x5A => char::from_u32(virtual_key).map(|ch| ch.to_ascii_lowercase()),
        _ => None,
    }
}

fn shift_digit_char(vk: u32) -> char {
    match vk {
        0x30 => ')',
        0x31 => '!',
        0x32 => '@',
        0x33 => '#',
        0x34 => '$',
        0x35 => '%',
        0x36 => '^',
        0x37 => '&',
        0x38 => '*',
        0x39 => '(',
        _ => unreachable!(),
    }
}

fn event_would_be_consumed(event: NativeKeyEvent, snapshot: &SessionSnapshot) -> bool {
    match event.keyval {
        SESSION_KEY_BACKSPACE | SESSION_KEY_ESCAPE | SESSION_KEY_RETURN | SESSION_KEY_SPACE => {
            !snapshot.raw_preedit.is_empty()
        }
        SESSION_KEY_LEFT | SESSION_KEY_RIGHT => snapshot.segmented_active,
        SESSION_KEY_UP | SESSION_KEY_DOWN => snapshot.segmented_active || !snapshot.candidates.is_empty(),
        _ => char::from_u32(event.keyval)
            .map(|ch| ch.is_ascii_graphic())
            .unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::input::key_convert::{ConvertedKey, STATE_CONTROL_MASK, VK_BACK, VK_RETURN, VK_SPACE};

    fn event(keyval: u32) -> NativeKeyEvent {
        NativeKeyEvent {
            keyval,
            keycode: keyval,
            state: 0,
        }
    }

    #[test]
    fn ctrl_a_passes_through_to_host_shortcut() {
        let input = windows_key_input_with_state(WPARAM(0x41), LPARAM(0), STATE_CONTROL_MASK);

        assert_eq!(convert_windows_key(input), ConvertedKey::PassThrough);
    }

    #[test]
    fn plain_a_remains_ime_handled() {
        let input = windows_key_input_with_state(WPARAM(0x41), LPARAM(0), 0);

        assert!(matches!(convert_windows_key(input), ConvertedKey::Event(_)));
    }

    #[test]
    fn plain_digit_is_ime_handled() {
        let input = windows_key_input_with_state(WPARAM(0x32), LPARAM(0), 0);

        assert!(matches!(convert_windows_key(input), ConvertedKey::Event(_)));
    }

    #[test]
    fn idle_editor_control_keys_pass_through() {
        let snapshot = SessionSnapshot::default();

        for keyval in [
            SESSION_KEY_BACKSPACE,
            SESSION_KEY_ESCAPE,
            SESSION_KEY_RETURN,
            SESSION_KEY_SPACE,
            SESSION_KEY_LEFT,
            SESSION_KEY_RIGHT,
            SESSION_KEY_UP,
            SESSION_KEY_DOWN,
        ] {
            assert!(!event_would_be_consumed(event(keyval), &snapshot));
        }
    }

    #[test]
    fn composing_editor_control_keys_are_handled() {
        let snapshot = SessionSnapshot {
            raw_preedit: "jea".to_owned(),
            preedit: "jea".to_owned(),
            candidates: vec!["candidate".to_owned()],
            ..SessionSnapshot::default()
        };

        for keyval in [
            SESSION_KEY_BACKSPACE,
            SESSION_KEY_ESCAPE,
            SESSION_KEY_RETURN,
            SESSION_KEY_SPACE,
        ] {
            assert!(event_would_be_consumed(event(keyval), &snapshot));
        }
        assert!(event_would_be_consumed(event(SESSION_KEY_UP), &snapshot));
        assert!(event_would_be_consumed(event(SESSION_KEY_DOWN), &snapshot));
    }

    #[test]
    fn printable_keys_still_start_composition_or_keycap_commit() {
        let snapshot = SessionSnapshot::default();

        assert!(event_would_be_consumed(event('a' as u32), &snapshot));
        assert!(event_would_be_consumed(event('2' as u32), &snapshot));
        assert!(event_would_be_consumed(event('@' as u32), &snapshot));
    }

    #[test]
    fn converted_idle_editor_control_keys_match_prediction() {
        for virtual_key in [VK_BACK, VK_RETURN, VK_SPACE] {
            let ConvertedKey::Event(event) = convert_windows_key(WindowsKeyInput {
                virtual_key,
                ..WindowsKeyInput::default()
            }) else {
                panic!("special key should convert");
            };
            assert!(!event_would_be_consumed(event, &SessionSnapshot::default()));
        }
    }

    #[test]
    fn shift_digit_chars_cover_full_row() {
        assert_eq!(shift_digit_char(0x30), ')');
        assert_eq!(shift_digit_char(0x31), '!');
        assert_eq!(shift_digit_char(0x32), '@');
        assert_eq!(shift_digit_char(0x33), '#');
        assert_eq!(shift_digit_char(0x34), '$');
        assert_eq!(shift_digit_char(0x35), '%');
        assert_eq!(shift_digit_char(0x36), '^');
        assert_eq!(shift_digit_char(0x37), '&');
        assert_eq!(shift_digit_char(0x38), '*');
        assert_eq!(shift_digit_char(0x39), '(');
    }
}
