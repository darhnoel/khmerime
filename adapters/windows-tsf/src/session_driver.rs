//! Pure Rust driver around `khmerime_session::ImeSession`.
//!
//! TSF/COM code should reduce native callbacks to `WindowsTsfCallback` values
//! and let this driver own the shared IME session.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

use khmerime_core::{DecoderConfig, Result as KhmerResult, Transliterator};
use khmerime_session::{HistoryStore, ImeSession, NativeKeyEvent, SegmentedPreviewMode, SessionCommand};

use crate::diagnostics::log;
use crate::{derive_render_state, map_callback_to_session_commands, WindowsRenderState, WindowsTsfCallback};

/// The first post-skeleton milestone for Windows adapter implementation.
pub const FIRST_IMPLEMENTATION_MILESTONE: &str = "pure Rust Windows session driver around ImeSession";

/// How far along the two-stage warmup this driver is.
///
/// Mirrors the IBus bridge's readiness states. `FullPending` exists because the
/// engine swap is deferred while a **Composition** is active — swapping the
/// transliterator mid-composition would re-decode the user's in-flight input
/// under a different engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverReadiness {
    PhaseA,
    FullPending,
    Full,
    Failed,
}

pub struct WindowsSessionDriver {
    session: ImeSession,
    full_warmup: Option<Receiver<Result<Transliterator, String>>>,
    pending_engine: Option<Transliterator>,
    readiness: DriverReadiness,
}

impl WindowsSessionDriver {
    pub fn new(session: ImeSession) -> Self {
        Self {
            session,
            full_warmup: None,
            pending_engine: None,
            readiness: DriverReadiness::Full,
        }
    }

    pub fn readiness(&self) -> DriverReadiness {
        self.readiness
    }

    /// Builds the Phase A engine on the calling thread and starts the full build.
    ///
    /// Phase A skips the stages that dominate cold start (`ranked_lexicon` and
    /// `search_index`), paying only the ~50 ms `parse_lexicon` +
    /// `parse_dictionary_image` prefix, so the driver exists before the first
    /// keystroke can arrive. That is what satisfies **Warmup Keystroke Capture**:
    /// there is no window in which `OnKeyDown` has no session to hand the key to.
    pub fn from_phase_a_data_traced() -> KhmerResult<Self> {
        let host = host_process_label();
        log(format!("[warmup-trace] phase_a.start host={host}"));

        let started = Instant::now();
        let transliterator = Transliterator::from_default_phase_a_data(DecoderConfig::shadow_interactive())?;
        let phase_a_ms = started.elapsed().as_secs_f64() * 1000.0;

        log(format!(
            "[warmup-trace] phase_a.end host={host} elapsed_ms={phase_a_ms:.2}"
        ));

        Ok(Self {
            session: ImeSession::new(transliterator, HashMap::new()),
            full_warmup: Some(spawn_full_engine_warmup()),
            pending_engine: None,
            readiness: DriverReadiness::PhaseA,
        })
    }

    /// Adopts the full engine when it lands, deferring while a composition is active.
    fn poll_full_warmup(&mut self) {
        let Some(receiver) = self.full_warmup.take() else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(engine)) => self.install_full_engine(engine),
            Ok(Err(error)) => {
                self.readiness = DriverReadiness::Failed;
                log(format!("[warmup-trace] full_warmup.failed error={error}"));
            }
            Err(TryRecvError::Empty) => {
                self.full_warmup = Some(receiver);
            }
            Err(TryRecvError::Disconnected) => {
                self.readiness = DriverReadiness::Failed;
                log("[warmup-trace] full_warmup.disconnected");
            }
        }
    }

    fn install_full_engine(&mut self, engine: Transliterator) {
        if self.session.composition_is_empty() {
            self.session
                .replace_engines(engine, None, SegmentedPreviewMode::Enabled);
            self.readiness = DriverReadiness::Full;
            log("[warmup-trace] full_upgrade.applied");
            return;
        }

        self.pending_engine = Some(engine);
        self.readiness = DriverReadiness::FullPending;
        log("[warmup-trace] full_upgrade.deferred active_composition=true");
    }

    /// Applies a deferred upgrade once the composition goes idle.
    fn maybe_complete_full_upgrade(&mut self) {
        if self.readiness != DriverReadiness::FullPending || !self.session.composition_is_empty() {
            return;
        }
        let Some(engine) = self.pending_engine.take() else {
            return;
        };
        self.session
            .replace_engines(engine, None, SegmentedPreviewMode::Enabled);
        self.readiness = DriverReadiness::Full;
        log("[warmup-trace] full_upgrade.applied_after_idle");
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
        self.poll_full_warmup();
        let mut last_result = Default::default();
        for command in map_callback_to_session_commands(&callback) {
            last_result = self.session.process_command(command);
        }
        // A commit may have just emptied the composition, releasing a deferred swap.
        self.maybe_complete_full_upgrade();
        derive_render_state(&self.session.snapshot(), &last_result)
    }

    pub fn process_command(&mut self, command: SessionCommand) -> WindowsRenderState {
        self.poll_full_warmup();
        let result = self.session.process_command(command);
        self.maybe_complete_full_upgrade();
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

/// Identifies which host application process is paying this warmup.
///
/// TSF loads the text service into each client app, so the same trace line
/// appears once per process. Without the exe name the log cannot distinguish
/// "slow once" from "slow in every app the user opens".
fn host_process_label() -> String {
    let exe = std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "unknown".to_owned());
    format!("{exe}/{}", std::process::id())
}

/// Builds the full engine off-thread while recording per-stage cold-start timings.
///
/// TSF loads this DLL into every host application process, so this cost is paid
/// again in each app the user types in. The stage breakdown distinguishes the
/// `parse_lexicon` + `parse_dictionary_image` prefix that Phase A also pays from
/// the `ranked_lexicon` and `search_index` stages that Phase A skips. See
/// `docs/adr/0001-warmup-must-not-leak-roman-into-the-document.md`.
fn spawn_full_engine_warmup() -> Receiver<Result<Transliterator, String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let host = host_process_label();
        log(format!("[warmup-trace] full.start host={host}"));

        let started = Instant::now();
        let result = Transliterator::from_default_shared_data_with_stage_logger(|stage, elapsed_ms| {
            log(format!("[warmup-trace] stage={stage} elapsed_ms={elapsed_ms:.2}"));
        })
        .map(|shared| Transliterator::from_shared_data_with_config(&shared, DecoderConfig::shadow_interactive()))
        .map_err(|err| err.to_string());
        let total_ms = started.elapsed().as_secs_f64() * 1000.0;

        log(format!(
            "[warmup-trace] full.end host={host} total_ms={total_ms:.2} ok={}",
            result.is_ok()
        ));
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

    fn fixture_engine(tsv: &str) -> Transliterator {
        Transliterator::from_tsv_str_with_config(tsv, DecoderConfig::shadow_interactive()).expect("fixture must parse")
    }

    /// A driver that behaves as if Phase A is live and the full build is in flight.
    fn phase_a_driver() -> WindowsSessionDriver {
        let mut driver = driver();
        driver.readiness = DriverReadiness::PhaseA;
        driver
    }

    #[test]
    fn full_engine_swaps_in_immediately_when_composition_is_idle() {
        let mut driver = phase_a_driver();
        driver.process_callback(WindowsTsfCallback::Activate);

        driver.install_full_engine(fixture_engine("jea\tswapped\n"));

        assert_eq!(driver.readiness(), DriverReadiness::Full);
    }

    #[test]
    fn full_engine_swap_defers_while_a_composition_is_active() {
        let mut driver = phase_a_driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "jea");
        assert!(!driver.session().composition_is_empty());

        driver.install_full_engine(fixture_engine("jea\tswapped\n"));

        // Swapping mid-composition would re-decode the user's in-flight input
        // under a different engine, so the upgrade must wait.
        assert_eq!(driver.readiness(), DriverReadiness::FullPending);
        assert!(driver.pending_engine.is_some());
    }

    #[test]
    fn deferred_full_engine_swap_applies_once_the_composition_clears() {
        let mut driver = phase_a_driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "jea");
        driver.install_full_engine(fixture_engine("jea\tswapped\n"));
        assert_eq!(driver.readiness(), DriverReadiness::FullPending);

        driver.process_key_event(key(SESSION_KEY_ESCAPE));

        assert_eq!(driver.readiness(), DriverReadiness::Full);
        assert!(driver.pending_engine.is_none());
    }

    #[test]
    fn deferred_full_engine_swap_applies_after_a_commit() {
        let mut driver = phase_a_driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "jea");
        driver.install_full_engine(fixture_engine("jea\tswapped\n"));

        let render = driver.process_key_event(key(SESSION_KEY_RETURN));

        assert!(render.commit_text.is_some());
        assert_eq!(driver.readiness(), DriverReadiness::Full);
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
