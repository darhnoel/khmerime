use std::io::{self, BufRead, Write};

use khmerime_core::{DecoderConfig, DecoderMode, Result as KhmerResult, Transliterator};
use khmerime_linux_ibus::{fallback_empty_snapshot_json, BridgeCommand, BridgeResponse, DesktopHistoryStore};
use khmerime_session::{HistoryStore, ImeSession, ImeSessionSnapshot, ImeSessionUpdate, InputMode};

fn build_response(session: &ImeSession, update: ImeSessionUpdate) -> BridgeResponse<ImeSessionSnapshot> {
    BridgeResponse {
        ok: true,
        consumed: update.consumed,
        commit_text: update.commit_text,
        history_changed: update.history_changed,
        snapshot: session.snapshot(),
        error: None,
    }
}

fn error_response(session: &ImeSession, message: impl Into<String>) -> BridgeResponse<ImeSessionSnapshot> {
    BridgeResponse {
        ok: false,
        consumed: false,
        commit_text: None,
        history_changed: false,
        snapshot: session.snapshot(),
        error: Some(message.into()),
    }
}

fn bootstrap_session(input_mode: InputMode) -> KhmerResult<ImeSession> {
    let store = DesktopHistoryStore;
    let transliterator = Transliterator::from_default_data_with_config(DecoderConfig::shadow_interactive())?;
    let commit_refiner = Transliterator::from_default_data_with_config(
        DecoderConfig::default()
            .with_mode(DecoderMode::Hybrid)
            .with_shadow_log(false),
    )?;
    let history = store.load().unwrap_or_default();
    Ok(ImeSession::new_with_commit_refiner_and_input_mode(
        transliterator,
        commit_refiner,
        history,
        input_mode,
    ))
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

fn apply_command(session: &mut ImeSession, command: BridgeCommand) -> (BridgeResponse<ImeSessionSnapshot>, bool) {
    match command {
        BridgeCommand::ProcessKeyEvent { keyval, keycode, state } => {
            let update = session.process_key_event(keyval, keycode, state);
            let mut response = build_response(session, update.clone());
            response.error = flush_history_if_changed(session, &update);
            (response, false)
        }
        BridgeCommand::RefineComposition { raw_preedit } => {
            session.apply_refined_candidate(&raw_preedit);
            (build_response(session, ImeSessionUpdate::default()), false)
        }
        BridgeCommand::SetInputMode { input_mode } => {
            session.set_input_mode(input_mode);
            (build_response(session, ImeSessionUpdate::default()), false)
        }
        BridgeCommand::ToggleInputMode => {
            session.toggle_input_mode();
            let update = ImeSessionUpdate {
                consumed: true,
                commit_text: None,
                history_changed: false,
            };
            (build_response(session, update), false)
        }
        BridgeCommand::FocusIn => {
            session.focus_in();
            (build_response(session, ImeSessionUpdate::default()), false)
        }
        BridgeCommand::FocusOut => {
            session.focus_out();
            (build_response(session, ImeSessionUpdate::default()), false)
        }
        BridgeCommand::Reset => {
            session.reset();
            (build_response(session, ImeSessionUpdate::default()), false)
        }
        BridgeCommand::Enable => {
            session.enable();
            (build_response(session, ImeSessionUpdate::default()), false)
        }
        BridgeCommand::Disable => {
            session.disable();
            (build_response(session, ImeSessionUpdate::default()), false)
        }
        BridgeCommand::SetCursorLocation { x, y, width, height } => {
            session.set_cursor_location(x, y, width, height);
            let update = ImeSessionUpdate {
                consumed: false,
                commit_text: None,
                history_changed: false,
            };
            (build_response(session, update), false)
        }
        BridgeCommand::Snapshot => {
            let update = ImeSessionUpdate {
                consumed: false,
                commit_text: None,
                history_changed: false,
            };
            (build_response(session, update), false)
        }
        BridgeCommand::Shutdown => (build_response(session, ImeSessionUpdate::default()), true),
    }
}

fn parse_initial_input_mode() -> Result<InputMode, String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let raw_value = if let Some(value) = arg.strip_prefix("--initial-input-mode=") {
            Some(value.to_owned())
        } else if arg == "--initial-input-mode" {
            args.next()
        } else {
            None
        };

        let Some(value) = raw_value else {
            continue;
        };
        return match value.as_str() {
            "roman" => Ok(InputMode::Roman),
            "nida" => Ok(InputMode::Nida),
            other => Err(format!("invalid --initial-input-mode value: {other}")),
        };
    }
    Ok(InputMode::Roman)
}

fn main() {
    let initial_input_mode = match parse_initial_input_mode() {
        Ok(input_mode) => input_mode,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };
    let mut session = match bootstrap_session(initial_input_mode) {
        Ok(mut session) => {
            session.set_cursor_location(0, 0, 0, 0);
            session
        }
        Err(err) => {
            eprintln!("failed to initialize transliterator: {err}");
            std::process::exit(2);
        }
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                let response = error_response(&session, format!("stdin read error: {err}"));
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
            Ok(command) => apply_command(&mut session, command),
            Err(err) => (error_response(&session, format!("invalid command: {err}")), false),
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
