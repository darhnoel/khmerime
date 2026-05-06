//! Pure Rust driver around `khmerime_session::ImeSession`.
//!
//! TSF/COM code should reduce native callbacks to `WindowsTsfCallback` values
//! and let this driver own the shared IME session.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use khmerime_core::{DecoderConfig, Result as KhmerResult, Transliterator};
use khmerime_session::{HistoryStore, ImeSession, NativeKeyEvent, SessionCommand};

use crate::{derive_render_state, map_callback_to_session_commands, WindowsRenderState, WindowsTsfCallback};

/// The first post-skeleton milestone for Windows adapter implementation.
pub const FIRST_IMPLEMENTATION_MILESTONE: &str = "pure Rust Windows session driver around ImeSession";

pub struct WindowsSessionDriver {
    session: ImeSession,
}

impl WindowsSessionDriver {
    pub fn new(session: ImeSession) -> Self {
        Self { session }
    }

    pub fn from_default_data() -> KhmerResult<Self> {
        let transliterator = Transliterator::from_default_data_with_config(DecoderConfig::shadow_interactive())?;
        Ok(Self::new(ImeSession::new(transliterator, HashMap::new())))
    }

    pub fn from_store<S: HistoryStore>(store: &S) -> Result<Self, S::Error> {
        let transliterator = Transliterator::from_default_data_with_config(DecoderConfig::shadow_interactive())
            .expect("default KhmerIME data must initialize");
        ImeSession::from_store(transliterator, store).map(Self::new)
    }

    pub fn process_callback(&mut self, callback: WindowsTsfCallback) -> WindowsRenderState {
        let mut last_result = Default::default();
        for command in map_callback_to_session_commands(&callback) {
            last_result = self.session.process_command(command);
        }
        derive_render_state(&self.session.snapshot(), &last_result)
    }

    pub fn process_command(&mut self, command: SessionCommand) -> WindowsRenderState {
        let result = self.session.process_command(command);
        derive_render_state(&self.session.snapshot(), &result)
    }

    pub fn process_key_event(&mut self, event: NativeKeyEvent) -> WindowsRenderState {
        self.process_callback(WindowsTsfCallback::KeyDown(event))
    }

    pub fn snapshot_render_state(&self) -> WindowsRenderState {
        derive_render_state(&self.session.snapshot(), &Default::default())
    }

    pub fn session(&self) -> &ImeSession {
        &self.session
    }
}

pub fn spawn_default_driver_warmup() -> Receiver<Result<WindowsSessionDriver, String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = WindowsSessionDriver::from_default_data()
            .map(|mut driver| {
                driver.process_callback(WindowsTsfCallback::Activate);
                driver
            })
            .map_err(|err| err.to_string());
        let _ = sender.send(result);
    });
    receiver
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use khmerime_core::{DecoderConfig, Transliterator};
    use khmerime_session::{ImeSession, SessionCommand};

    use super::*;
    use crate::input::key_convert::{
        convert_windows_key, ConvertedKey, WindowsKeyInput, SESSION_KEY_BACKSPACE, SESSION_KEY_ESCAPE,
        SESSION_KEY_RETURN, SESSION_KEY_SPACE, STATE_CONTROL_MASK, VK_BACK, VK_ESCAPE, VK_RETURN, VK_SPACE,
    };

    fn driver() -> WindowsSessionDriver {
        let fixture = "jea\tcandidate\nchea\tcandidate\nfoo\tfirst\nfoo\tsecond\n";
        let transliterator = Transliterator::from_tsv_str_with_config(fixture, DecoderConfig::shadow_interactive())
            .expect("fixture must parse");
        let session = ImeSession::new(transliterator, HashMap::new());
        WindowsSessionDriver::new(session)
    }

    fn key(keyval: u32) -> NativeKeyEvent {
        NativeKeyEvent {
            keyval,
            keycode: keyval,
            state: 0,
        }
    }

    fn type_ascii(driver: &mut WindowsSessionDriver, text: &str) -> WindowsRenderState {
        let mut render = driver.snapshot_render_state();
        for ch in text.chars() {
            render = driver.process_key_event(key(ch as u32));
        }
        render
    }

    #[test]
    fn activation_enables_and_focuses_session() {
        let mut driver = driver();
        let render = driver.process_callback(WindowsTsfCallback::Activate);

        assert!(!render.consumed);
        assert!(driver.session().snapshot().enabled);
        assert!(driver.session().snapshot().focused);
    }

    #[test]
    fn jea_enter_commits_candidate_once() {
        let mut driver = driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "jea");

        let render = driver.process_key_event(key(SESSION_KEY_RETURN));

        assert!(render.consumed);
        assert_eq!(render.commit_text.as_deref(), Some("candidate"));
        assert!(render.preedit.is_empty());
    }

    #[test]
    fn backspace_and_escape_update_preedit() {
        let mut driver = driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "je");

        let render = driver.process_key_event(key(SESSION_KEY_BACKSPACE));
        assert_eq!(render.preedit, "j");

        let render = driver.process_key_event(key(SESSION_KEY_ESCAPE));
        assert!(render.consumed);
        assert!(render.preedit.is_empty());
    }

    #[test]
    fn space_cycles_candidates() {
        let mut driver = driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "foo");

        let render = driver.process_key_event(key(SESSION_KEY_SPACE));

        assert!(render.consumed);
        assert_eq!(render.selected_index, Some(1));
    }

    #[test]
    fn number_key_selects_candidate_without_committing() {
        let mut driver = driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "foo");

        let render = driver.process_key_event(key('2' as u32));

        assert!(render.consumed);
        assert_eq!(render.selected_index, Some(1));
        assert!(render.commit_text.is_none());
    }

    #[test]
    fn ctrl_shortcut_is_not_sent_to_session() {
        let converted = convert_windows_key(WindowsKeyInput {
            virtual_key: 0x41,
            state: STATE_CONTROL_MASK,
            translated_char: Some('a'),
            ..WindowsKeyInput::default()
        });

        assert_eq!(converted, ConvertedKey::PassThrough);
    }

    #[test]
    fn converted_windows_keys_drive_session() {
        let mut driver = driver();
        driver.process_callback(WindowsTsfCallback::Activate);

        for (virtual_key, translated_char) in [(0x4A, Some('j')), (0x45, Some('e')), (0x41, Some('a'))] {
            let ConvertedKey::Event(event) = convert_windows_key(WindowsKeyInput {
                virtual_key,
                translated_char,
                ..WindowsKeyInput::default()
            }) else {
                panic!("printable key should convert");
            };
            driver.process_key_event(event);
        }

        let ConvertedKey::Event(enter) = convert_windows_key(WindowsKeyInput {
            virtual_key: VK_RETURN,
            ..WindowsKeyInput::default()
        }) else {
            panic!("enter should convert");
        };
        let render = driver.process_key_event(enter);

        assert_eq!(render.commit_text.as_deref(), Some("candidate"));
    }

    #[test]
    fn callback_commands_apply_cursor_location() {
        let mut driver = driver();
        let render = driver.process_callback(WindowsTsfCallback::CursorRectChanged(
            khmerime_session::CursorLocation {
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            },
        ));

        assert_eq!(render.cursor_location.x, 1);
        assert_eq!(render.cursor_location.height, 4);
    }

    #[test]
    fn deactivate_clears_active_composition() {
        let mut driver = driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "jea");

        let render = driver.process_callback(WindowsTsfCallback::Deactivate);

        assert!(render.preedit.is_empty());
        assert!(!driver.session().snapshot().enabled);
        assert!(!driver.session().snapshot().focused);
    }

    #[test]
    fn special_virtual_keys_convert_for_driver() {
        for (virtual_key, expected) in [
            (VK_BACK, SESSION_KEY_BACKSPACE),
            (VK_ESCAPE, SESSION_KEY_ESCAPE),
            (VK_SPACE, SESSION_KEY_SPACE),
            (VK_RETURN, SESSION_KEY_RETURN),
        ] {
            let ConvertedKey::Event(event) = convert_windows_key(WindowsKeyInput {
                virtual_key,
                ..WindowsKeyInput::default()
            }) else {
                panic!("special key should convert");
            };
            assert_eq!(event.keyval, expected);
        }
    }

    #[test]
    fn direct_command_processing_is_available_for_tsf_shell() {
        let mut driver = driver();
        let render = driver.process_command(SessionCommand::FocusIn);

        assert!(!render.consumed);
        assert!(driver.session().snapshot().focused);
    }
}
