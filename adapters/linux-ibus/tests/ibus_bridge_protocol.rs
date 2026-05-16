use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

use serde_json::Value;

fn bridge_path() -> &'static str {
    env!("CARGO_BIN_EXE_khmerime_ibus_bridge")
}

fn spawn_bridge() -> (Child, ChildStdin, BufReader<std::process::ChildStdout>) {
    spawn_bridge_with_args(&[])
}

fn spawn_bridge_with_args(args: &[&str]) -> (Child, ChildStdin, BufReader<std::process::ChildStdout>) {
    let mut command = Command::new(bridge_path());
    command.args(args);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn bridge");
    let stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    (child, stdin, BufReader::new(stdout))
}

fn spawn_full_bridge() -> (Child, ChildStdin, BufReader<std::process::ChildStdout>) {
    spawn_bridge_with_args(&["--synchronous-full-startup"])
}

fn spawn_full_bridge_deferred_preview() -> (Child, ChildStdin, BufReader<std::process::ChildStdout>) {
    spawn_bridge_with_args(&["--synchronous-full-startup", "--deferred-segmented-preview"])
}

fn send_command(stdin: &mut impl Write, command: &str) {
    writeln!(stdin, "{command}").expect("write command");
    stdin.flush().expect("flush command");
}

fn read_response(stdout: &mut BufReader<impl std::io::Read>) -> Value {
    let mut line = String::new();
    stdout.read_line(&mut line).expect("read response");
    serde_json::from_str(line.trim()).expect("valid json response")
}

fn shutdown_and_assert_ok(mut child: Child, stdin: &mut ChildStdin, stdout: &mut BufReader<impl std::io::Read>) {
    send_command(stdin, r#"{"cmd":"shutdown"}"#);
    let _ = read_response(stdout);
    let status = child.wait().expect("bridge exit");
    assert!(status.success());
}

fn send_ascii_text(stdin: &mut impl Write, stdout: &mut BufReader<impl std::io::Read>, text: &str) {
    for ch in text.chars() {
        send_command(
            stdin,
            &format!(
                r#"{{"cmd":"process_key_event","keyval":{},"keycode":0,"state":0}}"#,
                ch as u32
            ),
        );
        let _ = read_response(stdout);
    }
}

#[test]
fn bridge_defaults_to_roman_input_mode() {
    let (child, mut stdin, mut stdout) = spawn_bridge();

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let response = read_response(&mut stdout);
    assert_eq!(response["snapshot"]["input_mode"], Value::String("roman".to_owned()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_phase_a_snapshot_is_available_without_full_warmup() {
    let (child, mut stdin, mut stdout) = spawn_bridge_with_args(&["--disable-full-warmup"]);

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let response = read_response(&mut stdout);
    assert_eq!(response["readiness"], Value::String("phase_a".to_owned()));
    assert_eq!(response["snapshot"]["input_mode"], Value::String("roman".to_owned()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_phase_a_candidates_do_not_enable_segmented_preview() {
    let (child, mut stdin, mut stdout) = spawn_bridge_with_args(&["--disable-full-warmup"]);

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "nihjeasnadaiborkbrae");

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let response = read_response(&mut stdout);
    assert_eq!(response["readiness"], Value::String("phase_a".to_owned()));
    assert_eq!(
        response["snapshot"]["raw_preedit"],
        Value::String("nihjeasnadaiborkbrae".to_owned())
    );
    assert!(response["snapshot"]["candidates"]
        .as_array()
        .map(|items| !items.is_empty())
        .unwrap_or(false));
    assert_eq!(response["snapshot"]["segmented_active"], Value::Bool(false));
    assert_eq!(response["snapshot"]["segment_preview"], Value::Array(Vec::new()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_can_start_in_nida_input_mode() {
    let (child, mut stdin, mut stdout) = spawn_bridge_with_args(&["--initial-input-mode", "nida"]);

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let response = read_response(&mut stdout);
    assert_eq!(response["snapshot"]["input_mode"], Value::String("nida".to_owned()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_toggle_input_mode_clears_composition() {
    let (child, mut stdin, mut stdout) = spawn_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "jea");

    send_command(&mut stdin, r#"{"cmd":"toggle_input_mode"}"#);
    let response = read_response(&mut stdout);
    assert_eq!(response["consumed"], Value::Bool(true));
    assert_eq!(response["snapshot"]["input_mode"], Value::String("nida".to_owned()));
    assert_eq!(response["snapshot"]["raw_preedit"], Value::String(String::new()));
    assert_eq!(response["snapshot"]["candidates"], Value::Array(Vec::<Value>::new()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_nida_mode_commits_direct_khmer_key() {
    let (child, mut stdin, mut stdout) = spawn_bridge_with_args(&["--initial-input-mode", "nida"]);

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":107,"keycode":37,"state":0}"#,
    );
    let response = read_response(&mut stdout);
    assert_eq!(response["consumed"], Value::Bool(true));
    assert_eq!(response["commit_text"], Value::String("ក".to_owned()));
    assert_eq!(response["snapshot"]["raw_preedit"], Value::String(String::new()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_nida_mode_does_not_treat_caps_uppercase_as_shift() {
    let (child, mut stdin, mut stdout) = spawn_bridge_with_args(&["--initial-input-mode", "nida"]);

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65,"keycode":30,"state":0}"#,
    );
    let response = read_response(&mut stdout);
    assert_eq!(response["consumed"], Value::Bool(true));
    assert_eq!(response["commit_text"], Value::String("ា".to_owned()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_nida_mode_uses_nida_xml_shift_space_mapping() {
    let (child, mut stdin, mut stdout) = spawn_bridge_with_args(&["--initial-input-mode", "nida"]);

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":32,"keycode":57,"state":1}"#,
    );
    let response = read_response(&mut stdout);
    assert_eq!(response["consumed"], Value::Bool(true));
    assert_eq!(response["commit_text"], Value::String(" ".to_owned()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_nida_mode_uses_evdev_top_letter_row() {
    let (child, mut stdin, mut stdout) = spawn_bridge_with_args(&["--initial-input-mode", "nida"]);
    let keys = [
        (113, 16, "ឆ"),
        (119, 17, "ឹ"),
        (101, 18, "េ"),
        (114, 19, "រ"),
        (116, 20, "ត"),
        (121, 21, "យ"),
        (117, 22, "ុ"),
        (105, 23, "ិ"),
        (111, 24, "ោ"),
        (112, 25, "ផ"),
    ];

    for (keyval, keycode, expected) in keys {
        send_command(
            &mut stdin,
            &format!(r#"{{"cmd":"process_key_event","keyval":{keyval},"keycode":{keycode},"state":0}}"#),
        );
        let response = read_response(&mut stdout);
        assert_eq!(response["consumed"], Value::Bool(true));
        assert_eq!(response["commit_text"], Value::String(expected.to_owned()));
    }

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_nida_mode_does_not_map_backspace_or_enter_evdev_keycodes() {
    let (child, mut stdin, mut stdout) = spawn_bridge_with_args(&["--initial-input-mode", "nida"]);

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65288,"keycode":14,"state":0}"#,
    );
    let backspace = read_response(&mut stdout);
    assert_eq!(backspace["consumed"], Value::Bool(false));
    assert_eq!(backspace["commit_text"], Value::Null);

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65293,"keycode":28,"state":0}"#,
    );
    let enter = read_response(&mut stdout);
    assert_eq!(enter["consumed"], Value::Bool(false));
    assert_eq!(enter["commit_text"], Value::Null);

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_commits_raw_roman_when_no_candidate() {
    let (child, mut stdin, mut stdout) = spawn_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);

    for keyval in [96, 96, 96] {
        send_command(
            &mut stdin,
            &format!(r#"{{"cmd":"process_key_event","keyval":{keyval},"keycode":0,"state":0}}"#),
        );
        let _ = read_response(&mut stdout);
    }

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65293,"keycode":0,"state":0}"#,
    );
    let commit_response = read_response(&mut stdout);
    assert_eq!(commit_response["commit_text"], Value::String("```".to_owned()));
    assert_eq!(commit_response["consumed"], Value::Bool(true));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_commits_single_keycap_digit_immediately() {
    let (child, mut stdin, mut stdout) = spawn_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":49,"keycode":0,"state":0}"#,
    );
    let response = read_response(&mut stdout);
    assert_eq!(response["consumed"], Value::Bool(true));
    assert_eq!(response["commit_text"], Value::String("១".to_owned()));
    assert_eq!(response["history_changed"], Value::Bool(false));
    assert_eq!(response["snapshot"]["preedit"], Value::String(String::new()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_tracks_cursor_location_callback() {
    let (child, mut stdin, mut stdout) = spawn_bridge();

    send_command(
        &mut stdin,
        r#"{"cmd":"set_cursor_location","x":12,"y":34,"width":56,"height":78}"#,
    );
    let response = read_response(&mut stdout);
    assert_eq!(response["snapshot"]["cursor_location"]["x"], Value::from(12));
    assert_eq!(response["snapshot"]["cursor_location"]["y"], Value::from(34));
    assert_eq!(response["snapshot"]["cursor_location"]["width"], Value::from(56));
    assert_eq!(response["snapshot"]["cursor_location"]["height"], Value::from(78));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_exposes_candidate_display_metadata() {
    let (child, mut stdin, mut stdout) = spawn_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);

    for keyval in ['j' as u32, 'e' as u32, 'a' as u32] {
        send_command(
            &mut stdin,
            &format!(r#"{{"cmd":"process_key_event","keyval":{keyval},"keycode":0,"state":0}}"#),
        );
        let _ = read_response(&mut stdout);
    }

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let snapshot = read_response(&mut stdout);
    let candidates = snapshot["snapshot"]["candidates"]
        .as_array()
        .expect("candidates should be an array");
    let display = snapshot["snapshot"]["candidate_display"]
        .as_array()
        .expect("candidate_display should be an array");
    assert_eq!(display.len(), candidates.len());
    assert!(display.iter().any(|entry| entry["recommended"] == Value::Bool(true)));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_supports_segment_focus_and_full_phrase_commit() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);

    for keyval in [107, 104, 110, 104, 111, 109, 116, 111, 118] {
        send_command(
            &mut stdin,
            &format!(r#"{{"cmd":"process_key_event","keyval":{keyval},"keycode":0,"state":0}}"#),
        );
        let _ = read_response(&mut stdout);
    }

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let snapshot = read_response(&mut stdout);
    assert_eq!(snapshot["snapshot"]["segmented_active"], Value::Bool(true));
    assert_eq!(snapshot["snapshot"]["focused_segment_index"], Value::from(0));
    assert_eq!(
        snapshot["snapshot"]["raw_preedit"],
        Value::String("khnhomtov".to_owned())
    );
    assert!(snapshot["snapshot"]["segment_preview"]
        .as_array()
        .map(|items| !items.is_empty())
        .unwrap_or(false));

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65363,"keycode":0,"state":0}"#,
    );
    let moved = read_response(&mut stdout);
    assert_eq!(moved["consumed"], Value::Bool(true));
    assert_eq!(moved["snapshot"]["focused_segment_index"], Value::from(1));

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65293,"keycode":0,"state":0}"#,
    );
    let committed = read_response(&mut stdout);
    let commit_text = committed["commit_text"]
        .as_str()
        .expect("commit text should be present");
    assert!(!commit_text.is_empty());
    assert_ne!(commit_text, "khnhomtov");

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_refines_long_phrase_on_enter() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "nihjeasnadaiborkbrae");

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65293,"keycode":0,"state":0}"#,
    );
    let committed = read_response(&mut stdout);
    assert_eq!(committed["commit_text"], Value::String("នេះជាស្នាដៃបកប្រែ".to_owned()));
    assert_eq!(committed["consumed"], Value::Bool(true));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_deferred_visible_refinement_updates_long_phrase_candidate() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge_deferred_preview();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "nihjeasnadaiborkbrae");

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let live = read_response(&mut stdout);
    assert_eq!(live["snapshot"]["segmented_active"], Value::Bool(false));
    assert_ne!(
        live["snapshot"]["candidates"]
            .as_array()
            .and_then(|items| items.first()),
        Some(&Value::String("នេះជាស្នាដៃបកប្រែ".to_owned()))
    );

    send_command(
        &mut stdin,
        r#"{"cmd":"refine_composition","raw_preedit":"nihjeasnadaiborkbrae"}"#,
    );
    let refined = read_response(&mut stdout);
    assert_eq!(
        refined["snapshot"]["candidates"]
            .as_array()
            .and_then(|items| items.first()),
        Some(&Value::String("នេះជាស្នាដៃបកប្រែ".to_owned()))
    );

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65293,"keycode":0,"state":0}"#,
    );
    let committed = read_response(&mut stdout);
    assert_eq!(committed["commit_text"], Value::String("នេះជាស្នាដៃបកប្រែ".to_owned()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_deferred_preview_builds_synchronously_for_digit_selection() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge_deferred_preview();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "sophamongkul");

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let pre_digit = read_response(&mut stdout);
    assert_eq!(pre_digit["snapshot"]["segmented_active"], Value::Bool(false));

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":50,"keycode":0,"state":0}"#,
    );
    let after_digit = read_response(&mut stdout);
    assert_eq!(after_digit["snapshot"]["segmented_active"], Value::Bool(true));
    assert_eq!(after_digit["snapshot"]["preedit"], Value::String("សុភមង្គល".to_owned()));
    assert_eq!(
        after_digit["snapshot"]["segment_preview"][0]["output"],
        Value::String("សុភ".to_owned())
    );

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_deferred_preview_does_not_revert_user_segment_selection_on_refresh() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge_deferred_preview();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "sophamongkul");

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":50,"keycode":0,"state":0}"#,
    );
    let after_digit = read_response(&mut stdout);
    assert_eq!(
        after_digit["snapshot"]["segment_preview"][0]["output"],
        Value::String("សុភ".to_owned())
    );

    send_command(
        &mut stdin,
        r#"{"cmd":"refresh_segmented_preview","raw_preedit":"sophamongkul"}"#,
    );
    let _ = read_response(&mut stdout);

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let after_refresh = read_response(&mut stdout);
    assert_eq!(
        after_refresh["snapshot"]["segment_preview"][0]["output"],
        Value::String("សុភ".to_owned()),
        "stale debounced refresh must not overwrite a touched segment selection"
    );
    assert_eq!(after_refresh["snapshot"]["preedit"], Value::String("សុភមង្គល".to_owned()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_enter_commits_visible_default_when_hidden_refinement_disagrees() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "kasanmot");

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let snapshot = read_response(&mut stdout);
    assert_eq!(snapshot["snapshot"]["preedit"], Value::String("ការសន្មត".to_owned()));

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65293,"keycode":0,"state":0}"#,
    );
    let committed = read_response(&mut stdout);
    assert_eq!(committed["commit_text"], Value::String("ការសន្មត".to_owned()));
    assert_ne!(committed["commit_text"], Value::String("កសាងម៉ូត".to_owned()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_refinement_keeps_live_segmented_long_phrase_state() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "nihjeasnadaiborkbrae");

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let live = read_response(&mut stdout);
    assert_ne!(
        live["snapshot"]["candidates"]
            .as_array()
            .and_then(|items| items.first()),
        Some(&Value::String("នេះជាស្នាដៃបកប្រែ".to_owned()))
    );
    assert_eq!(live["snapshot"]["segmented_active"], Value::Bool(true));

    send_command(
        &mut stdin,
        r#"{"cmd":"refine_composition","raw_preedit":"nihjeasnadaiborkbrae"}"#,
    );
    let refined = read_response(&mut stdout);
    assert_eq!(
        refined["snapshot"]["candidates"]
            .as_array()
            .and_then(|items| items.first()),
        Some(&Value::String("នេះ".to_owned()))
    );
    assert_eq!(
        refined["snapshot"]["raw_preedit"],
        Value::String("nihjeasnadaiborkbrae".to_owned())
    );
    assert_eq!(refined["snapshot"]["segmented_active"], Value::Bool(true));
    assert_eq!(refined["snapshot"]["selected_index"], Value::from(0));

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65293,"keycode":0,"state":0}"#,
    );
    let committed = read_response(&mut stdout);
    assert_eq!(committed["commit_text"], Value::String("នេះជាស្នាដៃបកប្រែ".to_owned()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_ignores_stale_visible_refinement_request() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "nihjeasnadaiborkbrae");

    send_command(
        &mut stdin,
        r#"{"cmd":"refine_composition","raw_preedit":"nihjeasnadai"}"#,
    );
    let stale = read_response(&mut stdout);
    assert_ne!(
        stale["snapshot"]["candidates"]
            .as_array()
            .and_then(|items| items.first()),
        Some(&Value::String("នេះជាស្នាដៃបកប្រែ".to_owned()))
    );
    assert_eq!(
        stale["snapshot"]["raw_preedit"],
        Value::String("nihjeasnadaiborkbrae".to_owned())
    );

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_refinement_preserves_segment_focus() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "nihjeasnadaiborkbrae");

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65363,"keycode":0,"state":0}"#,
    );
    let moved = read_response(&mut stdout);
    assert_eq!(moved["consumed"], Value::Bool(true));
    assert_eq!(moved["snapshot"]["focused_segment_index"], Value::from(1));

    send_command(
        &mut stdin,
        r#"{"cmd":"refine_composition","raw_preedit":"nihjeasnadaiborkbrae"}"#,
    );
    let refined = read_response(&mut stdout);
    assert_eq!(refined["snapshot"]["segmented_active"], Value::Bool(true));
    assert_eq!(refined["snapshot"]["focused_segment_index"], Value::from(1));
    assert_ne!(
        refined["snapshot"]["candidates"]
            .as_array()
            .and_then(|items| items.first()),
        Some(&Value::String("នេះជាស្នាដៃបកប្រែ".to_owned()))
    );

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_commits_live_segmented_long_phrase_on_enter() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);
    send_ascii_text(&mut stdin, &mut stdout, "nihjeasnadaiborkbrae");

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let snapshot = read_response(&mut stdout);
    assert_eq!(snapshot["snapshot"]["segmented_active"], Value::Bool(true));

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65293,"keycode":0,"state":0}"#,
    );
    let committed = read_response(&mut stdout);
    assert_eq!(committed["commit_text"], Value::String("នេះជាស្នាដៃបកប្រែ".to_owned()));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}

#[test]
fn bridge_consumes_up_down_during_segmented_selection() {
    let (child, mut stdin, mut stdout) = spawn_full_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);

    for keyval in [107, 104, 110, 104, 111, 109, 116, 111, 118] {
        send_command(
            &mut stdin,
            &format!(r#"{{"cmd":"process_key_event","keyval":{keyval},"keycode":0,"state":0}}"#),
        );
        let _ = read_response(&mut stdout);
    }

    send_command(&mut stdin, r#"{"cmd":"snapshot"}"#);
    let snapshot = read_response(&mut stdout);
    assert_eq!(snapshot["snapshot"]["segmented_active"], Value::Bool(true));
    assert_eq!(snapshot["snapshot"]["focused_segment_index"], Value::from(0));

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65364,"keycode":0,"state":0}"#,
    );
    let down = read_response(&mut stdout);
    assert_eq!(down["consumed"], Value::Bool(true));
    assert_eq!(down["snapshot"]["segmented_active"], Value::Bool(true));
    assert_eq!(down["snapshot"]["focused_segment_index"], Value::from(0));

    send_command(
        &mut stdin,
        r#"{"cmd":"process_key_event","keyval":65362,"keycode":0,"state":0}"#,
    );
    let up = read_response(&mut stdout);
    assert_eq!(up["consumed"], Value::Bool(true));
    assert_eq!(up["snapshot"]["segmented_active"], Value::Bool(true));
    assert_eq!(up["snapshot"]["focused_segment_index"], Value::from(0));

    shutdown_and_assert_ok(child, &mut stdin, &mut stdout);
}
