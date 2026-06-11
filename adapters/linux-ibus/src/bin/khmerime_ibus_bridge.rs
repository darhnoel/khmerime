use std::io::{self, BufRead, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use khmerime_core::{DecoderConfig, DecoderMode, Result as KhmerResult, Transliterator};
use khmerime_linux_ibus::{
    fallback_empty_snapshot_json, BridgeCommand, BridgeReadiness, BridgeResponse, BridgeTimings, DesktopHistoryStore,
};
use khmerime_session::{
    HistoryStore, ImeSession, ImeSessionOptions, ImeSessionSnapshot, ImeSessionUpdate, InputMode, SegmentedPreviewMode,
};

// Short grace, not a wait: full warmup measured at ~2.8s in release builds, so
// blocking longer just freezes the UI without producing a better commit.
// Short words commit immediately from PhaseA; longer phrase commits get a tiny
// chance to catch a full refiner that's already about to land.
const LONG_ENTER_REFINER_GRACE: Duration = Duration::from_millis(5);
const ENTER_REFINER_MIN_RAW_CHARS: usize = 12;

struct FullEngines {
    live: Transliterator,
    visible_refiner: Transliterator,
    commit_refiner: Transliterator,
}

struct BridgeRuntime {
    session: ImeSession,
    readiness: BridgeReadiness,
    full_warmup: Option<Receiver<Result<FullEngines, String>>>,
    pending_engines: Option<FullEngines>,
    full_segmented_preview_mode: SegmentedPreviewMode,
}

impl BridgeRuntime {
    fn new(
        input_mode: InputMode,
        full_warmup: Option<Receiver<Result<FullEngines, String>>>,
        full_segmented_preview_mode: SegmentedPreviewMode,
    ) -> KhmerResult<Self> {
        let started = Instant::now();
        eprintln!("[ibus-startup] phase_a_session.start");
        let store = DesktopHistoryStore;
        let transliterator = Transliterator::from_default_phase_a_data(DecoderConfig::legacy())?;
        let history = store.load().unwrap_or_default();
        let session = ImeSession::builder(transliterator, history)
            .input_mode(input_mode)
            .options(ImeSessionOptions {
                segmented_preview: SegmentedPreviewMode::Disabled,
            })
            .build();
        eprintln!(
            "[ibus-startup] phase_a_session.end elapsed_ms={:.2}",
            started.elapsed().as_secs_f64() * 1000.0
        );
        Ok(Self {
            session,
            readiness: BridgeReadiness::PhaseA,
            full_warmup,
            pending_engines: None,
            full_segmented_preview_mode,
        })
    }

    fn new_full(input_mode: InputMode, full_segmented_preview_mode: SegmentedPreviewMode) -> KhmerResult<Self> {
        let started = Instant::now();
        eprintln!("[ibus-startup] full_session.start");
        let store = DesktopHistoryStore;
        let engines = build_full_engines()?;
        let history = store.load().unwrap_or_default();
        let mut session = ImeSession::builder(engines.live, history)
            .input_mode(input_mode)
            .visible_refiner(engines.visible_refiner)
            .commit_refiner(engines.commit_refiner)
            .options(ImeSessionOptions {
                segmented_preview: full_segmented_preview_mode,
            })
            .build();
        session.set_cursor_location(0, 0, 0, 0);
        eprintln!(
            "[ibus-startup] full_session.end elapsed_ms={:.2}",
            started.elapsed().as_secs_f64() * 1000.0
        );
        Ok(Self {
            session,
            readiness: BridgeReadiness::Full,
            full_warmup: None,
            pending_engines: None,
            full_segmented_preview_mode,
        })
    }

    fn poll_full_warmup(&mut self) {
        let Some(receiver) = self.full_warmup.take() else {
            return;
        };
        match receiver.try_recv() {
            Ok(result) => self.handle_full_warmup_result(result),
            Err(TryRecvError::Empty) => {
                self.full_warmup = Some(receiver);
            }
            Err(TryRecvError::Disconnected) => {
                self.readiness = BridgeReadiness::Failed;
                eprintln!("[ibus-startup] full_warmup.disconnected");
            }
        }
    }

    fn wait_for_full_refiner(&mut self, timeout: Duration) {
        let Some(receiver) = self.full_warmup.take() else {
            return;
        };
        eprintln!(
            "[ibus-startup] enter_wait_for_refiner.start timeout_ms={}",
            timeout.as_millis()
        );
        match receiver.recv_timeout(timeout) {
            Ok(result) => self.handle_full_warmup_result(result),
            Err(RecvTimeoutError::Timeout) => {
                eprintln!("[ibus-startup] enter_wait_for_refiner.timeout");
                self.full_warmup = Some(receiver);
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.readiness = BridgeReadiness::Failed;
                eprintln!("[ibus-startup] enter_wait_for_refiner.disconnected");
            }
        }
    }

    fn handle_full_warmup_result(&mut self, result: Result<FullEngines, String>) {
        match result {
            Ok(engines) => self.install_full_engines(engines),
            Err(error) => {
                self.readiness = BridgeReadiness::Failed;
                eprintln!("[ibus-startup] full_warmup.failed error={error}");
            }
        }
    }

    fn install_full_engines(&mut self, engines: FullEngines) {
        if self.session.composition_is_empty() {
            self.session.replace_engines_with_refiners(
                engines.live,
                Some(engines.visible_refiner),
                Some(engines.commit_refiner),
                self.full_segmented_preview_mode,
            );
            self.readiness = BridgeReadiness::Full;
            eprintln!("[ibus-startup] full_upgrade.applied");
            return;
        }

        self.pending_engines = Some(engines);
        self.readiness = BridgeReadiness::FullPending;
        eprintln!("[ibus-startup] full_upgrade.deferred active_composition=true");
    }

    fn maybe_complete_full_upgrade(&mut self) {
        if self.readiness != BridgeReadiness::FullPending || !self.session.composition_is_empty() {
            return;
        }
        let Some(engines) = self.pending_engines.take() else {
            return;
        };
        self.session.replace_engines_with_refiners(
            engines.live,
            Some(engines.visible_refiner),
            Some(engines.commit_refiner),
            self.full_segmented_preview_mode,
        );
        self.readiness = BridgeReadiness::Full;
        eprintln!("[ibus-startup] full_upgrade.applied_after_idle");
    }
}

fn build_response(
    snapshot: ImeSessionSnapshot,
    readiness: BridgeReadiness,
    update: ImeSessionUpdate,
) -> BridgeResponse<ImeSessionSnapshot> {
    BridgeResponse {
        ok: true,
        consumed: update.consumed,
        commit_text: update.commit_text,
        history_changed: update.history_changed,
        readiness,
        snapshot,
        error: None,
        timings: None,
    }
}

fn error_response(
    session: &ImeSession,
    readiness: BridgeReadiness,
    message: impl Into<String>,
) -> BridgeResponse<ImeSessionSnapshot> {
    BridgeResponse {
        ok: false,
        consumed: false,
        commit_text: None,
        history_changed: false,
        readiness,
        snapshot: session.snapshot(),
        error: Some(message.into()),
        timings: None,
    }
}

fn log_startup_stage_end(stage: &str, started: Instant) {
    eprintln!(
        "[ibus-startup] {stage}.end elapsed_ms={:.2}",
        started.elapsed().as_secs_f64() * 1000.0
    );
}

fn build_full_engines() -> KhmerResult<FullEngines> {
    let started = Instant::now();
    eprintln!("[ibus-startup] full_shared_data.start");
    let shared = Transliterator::from_default_shared_data_with_stage_logger(|stage, elapsed_ms| {
        eprintln!("[ibus-startup] full_shared_data.{stage}.end elapsed_ms={elapsed_ms:.2}");
    })?;
    log_startup_stage_end("full_shared_data", started);

    let started = Instant::now();
    eprintln!("[ibus-startup] full_live_engine.start");
    let live = Transliterator::from_shared_data_with_config(&shared, DecoderConfig::shadow_interactive());
    log_startup_stage_end("full_live_engine", started);

    let started = Instant::now();
    eprintln!("[ibus-startup] full_visible_refiner.start");
    let visible_refiner =
        Transliterator::from_shared_data_with_config(&shared, visible_refiner_config().with_mode(DecoderMode::Hybrid));
    log_startup_stage_end("full_visible_refiner", started);

    let started = Instant::now();
    eprintln!("[ibus-startup] full_commit_refiner.start");
    let commit_refiner = Transliterator::from_shared_data_with_config(&shared, commit_refiner_config());
    log_startup_stage_end("full_commit_refiner", started);

    Ok(FullEngines {
        live,
        visible_refiner,
        commit_refiner,
    })
}

fn visible_refiner_config() -> DecoderConfig {
    let mut config = DecoderConfig::shadow_interactive();
    config.wfst_max_latency_ms = 75;
    config
}

fn commit_refiner_config() -> DecoderConfig {
    let mut config = DecoderConfig::default()
        .with_mode(DecoderMode::Hybrid)
        .with_shadow_log(false);
    config.wfst_max_latency_ms = 150;
    config
}

fn start_full_warmup(disabled: bool) -> Option<Receiver<Result<FullEngines, String>>> {
    if disabled {
        eprintln!("[ibus-startup] full_warmup.disabled");
        return None;
    }
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let started = Instant::now();
        eprintln!("[ibus-startup] full_warmup.start");
        let result = build_full_engines().map_err(|error| error.to_string());
        eprintln!(
            "[ibus-startup] full_warmup.end elapsed_ms={:.2}",
            started.elapsed().as_secs_f64() * 1000.0
        );
        let _ = sender.send(result);
    });
    Some(receiver)
}

fn needs_segmented_preview_for_keyval(keyval: u32) -> bool {
    matches!(
        keyval,
        0x0020                                       // Space
            | 0xFF51 | 0xFF52 | 0xFF53 | 0xFF54      // Left, Up, Right, Down
            | 0xFF89 | 0xFF96 | 0xFF98 | 0xFF99      // KP Tab/Left/Up/Right
            | 0x0031..=0x0039                        // '1'..='9' (top row digits)
            | 0xFFB1..=0xFFB9                        // KP_1..KP_9
    )
}

fn enter_refiner_grace(raw_preedit: &str) -> Option<Duration> {
    (raw_preedit.chars().count() >= ENTER_REFINER_MIN_RAW_CHARS).then_some(LONG_ENTER_REFINER_GRACE)
}

fn flush_history_if_changed(session: &ImeSession, update: &ImeSessionUpdate) -> Option<String> {
    if !update.history_changed {
        return None;
    }
    let store = DesktopHistoryStore;
    session
        .save_history(&store)
        .err()
        .map(|err| format!("history save failed: {err}"))
}

fn apply_command(runtime: &mut BridgeRuntime, command: BridgeCommand) -> (BridgeResponse<ImeSessionSnapshot>, bool) {
    let cmd_started = Instant::now();
    let emit_timings = matches!(
        command,
        BridgeCommand::ProcessKeyEvent { .. }
            | BridgeCommand::RefineComposition { .. }
            | BridgeCommand::RefreshSegmentedPreview { .. }
    );
    let mut t_wait = 0.0_f64;
    let mut t_seg = 0.0_f64;
    let mut t_proc = 0.0_f64;
    let mut t_hist = 0.0_f64;

    runtime.poll_full_warmup();

    let is_enter = matches!(
        command,
        BridgeCommand::ProcessKeyEvent {
            keyval: 0xFF0D | 0xFF8D,
            ..
        }
    );
    if is_enter && runtime.readiness == BridgeReadiness::PhaseA {
        let started = Instant::now();
        if let Some(grace) = enter_refiner_grace(runtime.session.composition_raw()) {
            runtime.wait_for_full_refiner(grace);
        }
        t_wait = started.elapsed().as_secs_f64() * 1000.0;
    }
    let needs_sync_segmented_preview = matches!(
        command,
        BridgeCommand::ProcessKeyEvent { keyval, .. } if needs_segmented_preview_for_keyval(keyval)
    );
    if needs_sync_segmented_preview
        && runtime.full_segmented_preview_mode == SegmentedPreviewMode::Deferred
        && !runtime.session.segmented_preview_active()
        && !runtime.session.composition_is_empty()
    {
        let started = Instant::now();
        let raw = runtime.session.composition_raw().to_owned();
        runtime.session.refresh_segmented_preview(&raw);
        t_seg = started.elapsed().as_secs_f64() * 1000.0;
    }

    let mut response_error = None;
    let (update, should_exit) = {
        let session = &mut runtime.session;
        match command {
            BridgeCommand::ProcessKeyEvent { keyval, keycode, state } => {
                let started = Instant::now();
                let update = session.process_key_event(keyval, keycode, state);
                t_proc = started.elapsed().as_secs_f64() * 1000.0;
                let hist_started = Instant::now();
                response_error = flush_history_if_changed(session, &update);
                t_hist = hist_started.elapsed().as_secs_f64() * 1000.0;
                (update, false)
            }
            BridgeCommand::RefineComposition { raw_preedit } => {
                let started = Instant::now();
                session.apply_refined_candidate(&raw_preedit);
                t_proc = started.elapsed().as_secs_f64() * 1000.0;
                (ImeSessionUpdate::default(), false)
            }
            BridgeCommand::RefreshSegmentedPreview { raw_preedit } => {
                let started = Instant::now();
                session.refresh_segmented_preview(&raw_preedit);
                t_proc = started.elapsed().as_secs_f64() * 1000.0;
                (ImeSessionUpdate::default(), false)
            }
            BridgeCommand::SetInputMode { input_mode } => {
                session.set_input_mode(input_mode);
                (ImeSessionUpdate::default(), false)
            }
            BridgeCommand::ToggleInputMode => {
                session.toggle_input_mode();
                (
                    ImeSessionUpdate {
                        consumed: true,
                        commit_text: None,
                        history_changed: false,
                    },
                    false,
                )
            }
            BridgeCommand::FocusIn => {
                session.focus_in();
                (ImeSessionUpdate::default(), false)
            }
            BridgeCommand::FocusOut => {
                session.focus_out();
                (ImeSessionUpdate::default(), false)
            }
            BridgeCommand::Reset => {
                session.reset();
                (ImeSessionUpdate::default(), false)
            }
            BridgeCommand::Enable => {
                session.enable();
                (ImeSessionUpdate::default(), false)
            }
            BridgeCommand::Disable => {
                session.disable();
                (ImeSessionUpdate::default(), false)
            }
            BridgeCommand::SetCursorLocation { x, y, width, height } => {
                session.set_cursor_location(x, y, width, height);
                (
                    ImeSessionUpdate {
                        consumed: false,
                        commit_text: None,
                        history_changed: false,
                    },
                    false,
                )
            }
            BridgeCommand::Snapshot => (
                ImeSessionUpdate {
                    consumed: false,
                    commit_text: None,
                    history_changed: false,
                },
                false,
            ),
            BridgeCommand::Shutdown => (ImeSessionUpdate::default(), true),
        }
    };

    runtime.poll_full_warmup();
    runtime.maybe_complete_full_upgrade();
    let snap_started = Instant::now();
    let snapshot = runtime.session.snapshot();
    let t_snap = snap_started.elapsed().as_secs_f64() * 1000.0;
    let mut response = build_response(snapshot, runtime.readiness, update);
    response.error = response_error;
    if emit_timings {
        let total = cmd_started.elapsed().as_secs_f64() * 1000.0;
        response.timings = Some(BridgeTimings {
            wait_refiner_ms: t_wait,
            sync_segpreview_ms: t_seg,
            process_event_ms: t_proc,
            history_save_ms: t_hist,
            snapshot_ms: t_snap,
            total_ms: total,
        });
    }
    (response, should_exit)
}

struct BridgeOptions {
    initial_input_mode: InputMode,
    disable_full_warmup: bool,
    synchronous_full_startup: bool,
    deferred_segmented_preview: bool,
}

fn parse_bridge_options() -> Result<BridgeOptions, String> {
    let mut args = std::env::args().skip(1);
    let mut initial_input_mode = InputMode::Roman;
    let mut disable_full_warmup = false;
    let mut synchronous_full_startup = false;
    let mut deferred_segmented_preview = false;
    while let Some(arg) = args.next() {
        let raw_value = if let Some(value) = arg.strip_prefix("--initial-input-mode=") {
            Some(value.to_owned())
        } else if arg == "--initial-input-mode" {
            args.next()
        } else {
            None
        };

        let Some(value) = raw_value else {
            if arg == "--disable-full-warmup" {
                disable_full_warmup = true;
            } else if arg == "--synchronous-full-startup" {
                synchronous_full_startup = true;
            } else if arg == "--deferred-segmented-preview" {
                deferred_segmented_preview = true;
            }
            continue;
        };
        initial_input_mode = match value.as_str() {
            "roman" => InputMode::Roman,
            "nida" => InputMode::Nida,
            other => return Err(format!("invalid --initial-input-mode value: {other}")),
        };
    }
    Ok(BridgeOptions {
        initial_input_mode,
        disable_full_warmup,
        synchronous_full_startup,
        deferred_segmented_preview,
    })
}

fn main() {
    eprintln!("[ibus-startup] bridge_process.start");
    let options = match parse_bridge_options() {
        Ok(options) => options,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };
    let full_segmented_preview_mode = if options.deferred_segmented_preview {
        SegmentedPreviewMode::Deferred
    } else {
        SegmentedPreviewMode::Enabled
    };
    let mut runtime = if options.synchronous_full_startup {
        match BridgeRuntime::new_full(options.initial_input_mode, full_segmented_preview_mode) {
            Ok(runtime) => runtime,
            Err(err) => {
                eprintln!("failed to initialize transliterator: {err}");
                std::process::exit(2);
            }
        }
    } else {
        match BridgeRuntime::new(options.initial_input_mode, None, full_segmented_preview_mode) {
            Ok(mut runtime) => {
                runtime.session.set_cursor_location(0, 0, 0, 0);
                runtime
            }
            Err(err) => {
                eprintln!("failed to initialize transliterator: {err}");
                std::process::exit(2);
            }
        }
    };
    if !options.synchronous_full_startup {
        runtime.full_warmup = start_full_warmup(options.disable_full_warmup);
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                let response = error_response(&runtime.session, runtime.readiness, format!("stdin read error: {err}"));
                let _ = writeln!(stdout, "{}", serde_json::to_string(&response).unwrap_or_default());
                let _ = stdout.flush();
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let parsed = serde_json::from_str::<BridgeCommand>(&line);
        let (response, should_exit) = match parsed {
            Ok(command) => apply_command(&mut runtime, command),
            Err(err) => (
                error_response(&runtime.session, runtime.readiness, format!("invalid command: {err}")),
                false,
            ),
        };

        let payload = serde_json::to_string(&response)
            .unwrap_or_else(|err| fallback_empty_snapshot_json(format!("serialization error: {err}")).to_string());

        if writeln!(stdout, "{payload}").is_err() {
            break;
        }
        if stdout.flush().is_err() {
            break;
        }
        if should_exit {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{enter_refiner_grace, LONG_ENTER_REFINER_GRACE};

    #[test]
    fn short_enter_commits_do_not_wait_for_full_refiner() {
        assert_eq!(enter_refiner_grace("jea"), None);
        assert_eq!(enter_refiner_grace("khnhomtov"), None);
    }

    #[test]
    fn long_enter_commits_get_only_tiny_refiner_grace() {
        assert_eq!(enter_refiner_grace("jeanggettengos"), Some(LONG_ENTER_REFINER_GRACE));
        assert_eq!(
            enter_refiner_grace("nihjeasnadaiborkbrae"),
            Some(LONG_ENTER_REFINER_GRACE)
        );
    }
}
