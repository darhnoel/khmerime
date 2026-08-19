//! Pure Rust driver around `khmerime_session::ImeSession`.
//!
//! TSF/COM code should reduce native callbacks to `WindowsTsfCallback` values
//! and let this driver own the shared IME session.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

use khmerime_core::{DecoderConfig, Result as KhmerResult, SpanProposalMode, Transliterator};
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

/// The engines produced by the full warmup.
///
/// Grouped so the swap is atomic: the live engine and its **Visible Refiner** must be installed
/// together, or a refine could run against a different engine than the one that produced the
/// composition.
pub struct FullEngines {
    pub live: Transliterator,
    /// `None` when unarmed — see [`visible_refiner_config`].
    pub visible_refiner: Option<Transliterator>,
}

pub struct WindowsSessionDriver {
    session: ImeSession,
    full_warmup: Option<Receiver<Result<FullEngines, String>>>,
    pending_engines: Option<FullEngines>,
    readiness: DriverReadiness,
}

impl WindowsSessionDriver {
    pub fn new(session: ImeSession) -> Self {
        Self {
            session,
            full_warmup: None,
            pending_engines: None,
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
            pending_engines: None,
            readiness: DriverReadiness::PhaseA,
        })
    }

    /// Adopts the full engine when it lands, deferring while a composition is active.
    fn poll_full_warmup(&mut self) {
        let Some(receiver) = self.full_warmup.take() else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(engines)) => self.install_full_engines(engines),
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

    fn install_full_engines(&mut self, engines: FullEngines) {
        if self.session.composition_is_empty() {
            self.apply_full_engines(engines);
            log("[warmup-trace] full_upgrade.applied");
            return;
        }

        self.pending_engines = Some(engines);
        self.readiness = DriverReadiness::FullPending;
        log("[warmup-trace] full_upgrade.deferred active_composition=true");
    }

    fn apply_full_engines(&mut self, engines: FullEngines) {
        self.session.replace_engines_with_refiners(
            engines.live,
            engines.visible_refiner,
            None,
            SegmentedPreviewMode::Enabled,
        );
        self.readiness = DriverReadiness::Full;
    }

    /// Applies a deferred upgrade once the composition goes idle.
    fn maybe_complete_full_upgrade(&mut self) {
        if self.readiness != DriverReadiness::FullPending || !self.session.composition_is_empty() {
            return;
        }
        let Some(engines) = self.pending_engines.take() else {
            return;
        };
        self.apply_full_engines(engines);
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
        if let WindowsTsfCallback::KeyDown(event) = callback {
            return self.process_key_event(event);
        }
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
        self.poll_full_warmup();
        let snapshot = self.session.snapshot();
        let surface = crate::render::candidate_surface::CandidateSurface::from_snapshot(&snapshot);
        let result = if let Some(command) = surface.command_for_key(event.keyval) {
            self.session.process_command(command)
        } else {
            self.session.process_command(SessionCommand::ProcessKeyEvent(event))
        };
        self.maybe_complete_full_upgrade();
        derive_render_state(&self.session.snapshot(), &result)
    }

    /// Runs the **Visible Refiner** against the live **Composition**.
    ///
    /// Routing is the part worth hiding: a **Segmented Session** refreshes its preview, while a
    /// flat composition refines its **Candidate List**. Callers should not have to know which
    /// shape the composition is in, only that they want it refined.
    ///
    /// Both shared entry points already reject a stale `raw_preedit`, a touched selection, and a
    /// non-roman input mode, so this deliberately re-checks none of that — duplicating those
    /// guards here is how the two copies drift apart.
    pub fn refine_now(&mut self) -> bool {
        let raw = self.session.composition_raw().to_owned();
        if raw.is_empty() {
            return false;
        }
        if self.session.segmented_preview_active() {
            self.session.refresh_segmented_preview(&raw)
        } else {
            self.session.apply_refined_candidate(&raw).is_some()
        }
    }

    pub fn snapshot_render_state(&self) -> WindowsRenderState {
        derive_render_state(&self.session.snapshot(), &Default::default())
    }

    pub fn session(&self) -> &ImeSession {
        &self.session
    }
}

/// Arming switch for the span-proposal seam, matching the IBus bridge and macOS IMK contract.
///
/// `model` resolves to a provider registered by a separate, private build; `static-test` is the
/// deterministic in-tree provider used by tests. Anything else — including unset — leaves the seam
/// inert, so the public build is the pure lookup + fuzzy engine (ADR-0016).
pub const SPAN_PROPOSALS_ENV: &str = "KHMERIME_SPAN_PROPOSALS";

pub fn span_proposal_mode_for(value: Option<&str>) -> SpanProposalMode {
    match value {
        Some("model") => SpanProposalMode::Model,
        Some("static-test") => SpanProposalMode::StaticTest,
        _ => SpanProposalMode::Disabled,
    }
}

/// The **Visible Refiner** config for this arming value, or `None` to build no refiner at all.
///
/// Returning `None` when unarmed is deliberate and differs from the IBus bridge, which always
/// builds a visible refiner (Hybrid when unarmed). Windows has never had one, so constructing it
/// unconditionally would change ranking for every existing user; the free build must stay exactly
/// as it is.
pub fn visible_refiner_config(value: Option<&str>) -> Option<DecoderConfig> {
    let mode = span_proposal_mode_for(value);
    if mode == SpanProposalMode::Disabled {
        return None;
    }
    let mut config = DecoderConfig::shadow_interactive().with_span_proposal_mode(mode);
    // The stock 250 ms budget is sized for a deterministic refiner. Neural inference costs
    // ~135-150 ms before the WFST does any work, so decodes carrying a model proposal blow the
    // budget and are discarded wholesale — indistinguishable from the model having nothing to say.
    // The refine is debounced and off the keystroke path, so the headroom is free.
    config.wfst_max_latency_ms = MODEL_REFINE_MAX_LATENCY_MS;
    Some(config)
}

/// Budget for the debounced **Visible Refiner** once a provider is armed.
const MODEL_REFINE_MAX_LATENCY_MS: u64 = 2_000;

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
fn spawn_full_engine_warmup() -> Receiver<Result<FullEngines, String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let host = host_process_label();
        log(format!("[warmup-trace] full.start host={host}"));

        let started = Instant::now();
        let result = Transliterator::from_default_shared_data_with_stage_logger(|stage, elapsed_ms| {
            log(format!("[warmup-trace] stage={stage} elapsed_ms={elapsed_ms:.2}"));
        })
        .map(|shared| {
            // Both engines are cheap clones over one SharedTransliteratorData, so the refiner adds
            // no build cost beyond its own decoder config.
            let live = Transliterator::from_shared_data_with_config(&shared, DecoderConfig::shadow_interactive());
            let visible_refiner = visible_refiner_config(std::env::var(SPAN_PROPOSALS_ENV).ok().as_deref())
                .map(|config| Transliterator::from_shared_data_with_config(&shared, config));
            log(format!(
                "[warmup-trace] span_provider_armed={}",
                visible_refiner.is_some()
            ));
            FullEngines { live, visible_refiner }
        })
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
        convert_windows_key, ConvertedKey, WindowsKeyInput, SESSION_KEY_BACKSPACE, SESSION_KEY_DOWN,
        SESSION_KEY_ESCAPE, SESSION_KEY_RETURN, SESSION_KEY_SPACE, SESSION_KEY_TAB, STATE_CONTROL_MASK, VK_BACK,
        VK_ESCAPE, VK_RETURN, VK_SPACE,
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

    // An unarmed Windows build must stay byte-identical to today's free behaviour. Linux always
    // builds a visible refiner (Hybrid when unarmed), but Windows has never had one, so switching
    // it on unconditionally would change ranking for every existing user. No arming, no refiner.
    #[test]
    fn an_unarmed_build_has_no_visible_refiner() {
        assert!(visible_refiner_config(None).is_none());
        assert!(visible_refiner_config(Some("")).is_none());
    }

    // 75 ms is right for a deterministic refiner but starves neural inference, which costs
    // ~135-150 ms on its own. With the default budget every model-carrying decode times out and is
    // discarded wholesale, so the model's answer never reaches the user and the failure looks
    // exactly like "the model had nothing to say". The refine is debounced and off the keystroke
    // path, so it can afford the headroom.
    #[test]
    fn an_armed_refiner_gets_a_budget_neural_inference_can_meet() {
        let config = visible_refiner_config(Some("model")).expect("model arming builds a refiner");

        assert!(
            config.wfst_max_latency_ms >= 2_000,
            "armed refiner budget too tight for inference: {}ms",
            config.wfst_max_latency_ms
        );
    }

    // The refiner has to arrive with the full engine, not with Phase A: Phase A exists to accept
    // the first keystroke in ~15 ms, and building a second engine there would put the cost straight
    // back. Attaching it during the swap keeps cold start unchanged.
    #[test]
    fn an_armed_driver_attaches_a_visible_refiner_when_the_full_engine_lands() {
        let mut driver = phase_a_driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        assert!(!driver.session().visible_refiner_active());

        driver.install_full_engines(FullEngines {
            live: fixture_engine(
                "jea	candidate
",
            ),
            visible_refiner: Some(fixture_engine(
                "jea	candidate
",
            )),
        });

        assert!(driver.session().visible_refiner_active());
    }

    fn fixture_engine_with(tsv: &str, config: DecoderConfig) -> Transliterator {
        Transliterator::from_tsv_str_with_config(tsv, config).expect("fixture must parse")
    }

    /// A driver whose refiner is armed with the deterministic in-tree provider.
    ///
    /// Uses the real Lexicon rather than a fixture: span proposals are assembled by the WFST
    /// lattice, which a two-entry fixture cannot feed, so a fixture would report "no model
    /// candidate" for the wrong reason. Both engines are cheap clones over one
    /// SharedTransliteratorData, so this pays a single build.
    ///
    /// `qzx -> គហិបតី` is a StaticTest row chosen precisely because it is absent from the Lexicon:
    /// only the provider can produce it, so seeing it proves the refine reached the span-proposal
    /// seam rather than just re-ranking Lexicon entries.
    fn statically_armed_driver() -> WindowsSessionDriver {
        let shared = Transliterator::from_default_shared_data().expect("default data must build");
        let live = Transliterator::from_shared_data_with_config(&shared, DecoderConfig::shadow_interactive());
        let refiner_config = visible_refiner_config(Some("static-test")).expect("static-test arms a refiner");
        let visible_refiner = Transliterator::from_shared_data_with_config(&shared, refiner_config);

        let mut driver = WindowsSessionDriver::new(ImeSession::new(
            Transliterator::from_shared_data_with_config(&shared, DecoderConfig::shadow_interactive()),
            HashMap::new(),
        ));
        driver.install_full_engines(FullEngines {
            live,
            visible_refiner: Some(visible_refiner),
        });
        driver
    }

    // The refine is what makes the provider reachable at all: arming builds the refiner, but
    // nothing consults it until something asks for a refinement off the keystroke path.
    #[test]
    fn a_refine_surfaces_a_model_candidate_the_lexicon_cannot_produce() {
        let mut driver = statically_armed_driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "qzx");

        driver.refine_now();

        let render = driver.snapshot_render_state();
        let candidates = render.candidate_surface.rows();
        assert!(
            candidates.iter().any(|row| row.contains('គ')),
            "refine did not surface the model candidate; rows={candidates:?}"
        );
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

        driver.install_full_engines(FullEngines {
            live: fixture_engine("jea\tswapped\n"),
            visible_refiner: None,
        });

        assert_eq!(driver.readiness(), DriverReadiness::Full);
    }

    #[test]
    fn full_engine_swap_defers_while_a_composition_is_active() {
        let mut driver = phase_a_driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "jea");
        assert!(!driver.session().composition_is_empty());

        driver.install_full_engines(FullEngines {
            live: fixture_engine("jea\tswapped\n"),
            visible_refiner: None,
        });

        // Swapping mid-composition would re-decode the user's in-flight input
        // under a different engine, so the upgrade must wait.
        assert_eq!(driver.readiness(), DriverReadiness::FullPending);
        assert!(driver.pending_engines.is_some());
    }

    #[test]
    fn deferred_full_engine_swap_applies_once_the_composition_clears() {
        let mut driver = phase_a_driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "jea");
        driver.install_full_engines(FullEngines {
            live: fixture_engine("jea\tswapped\n"),
            visible_refiner: None,
        });
        assert_eq!(driver.readiness(), DriverReadiness::FullPending);

        driver.process_key_event(key(SESSION_KEY_ESCAPE));

        assert_eq!(driver.readiness(), DriverReadiness::Full);
        assert!(driver.pending_engines.is_none());
    }

    #[test]
    fn deferred_full_engine_swap_applies_after_a_commit() {
        let mut driver = phase_a_driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "jea");
        driver.install_full_engines(FullEngines {
            live: fixture_engine("jea\tswapped\n"),
            visible_refiner: None,
        });

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
        assert_eq!(render.candidate_surface.selected_index(), Some(1));
    }

    #[test]
    fn windows_cycles_whole_phrases_until_tab_enters_segment_edit() {
        let mut driver = WindowsSessionDriver::from_default_data().expect("default data");
        driver.process_callback(WindowsTsfCallback::Activate);
        let initial = type_ascii(&mut driver, "khnhomttov");
        assert_eq!(
            initial.candidate_surface.mode(),
            crate::render::candidate_surface::CandidateSurfaceMode::Phrase
        );
        assert!(initial.candidate_surface.rows().len() > 1);

        let phrase = driver.process_key_event(key(SESSION_KEY_DOWN));
        assert_eq!(phrase.candidate_surface.selected_index(), Some(1));

        let best_phrase = driver.process_key_event(key(crate::input::key_convert::SESSION_KEY_UP));
        assert_eq!(best_phrase.candidate_surface.selected_index(), Some(0));

        let segment = driver.process_key_event(key(SESSION_KEY_TAB));
        assert_eq!(
            segment.candidate_surface.mode(),
            crate::render::candidate_surface::CandidateSurfaceMode::Segment
        );
        let cycled_word = driver.process_key_event(key(SESSION_KEY_SPACE));
        assert_eq!(cycled_word.commit_text, None);
        assert_eq!(
            cycled_word.candidate_surface.mode(),
            crate::render::candidate_surface::CandidateSurfaceMode::Segment
        );
        assert!(!cycled_word.preedit.is_empty());
    }

    #[test]
    fn number_key_selects_candidate_without_committing() {
        let mut driver = driver();
        driver.process_callback(WindowsTsfCallback::Activate);
        type_ascii(&mut driver, "foo");

        let render = driver.process_key_event(key('2' as u32));

        assert!(render.consumed);
        assert_eq!(render.candidate_surface.selected_index(), Some(1));
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
