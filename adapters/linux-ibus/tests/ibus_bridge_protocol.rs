use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

use serde_json::Value;

fn bridge_path() -> &'static str {
    env!("CARGO_BIN_EXE_khmerime_ibus_bridge")
}

fn spawn_bridge() -> (Child, ChildStdin, BufReader<std::process::ChildStdout>) {
    let mut child = Command::new(bridge_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn bridge");
    let stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    (child, stdin, BufReader::new(stdout))
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

#[test]
fn bridge_commits_raw_roman_when_no_candidate() {
    let (child, mut stdin, mut stdout) = spawn_bridge();

    send_command(&mut stdin, r#"{"cmd":"focus_in"}"#);
    let _ = read_response(&mut stdout);

    for keyval in [35, 36, 37] {
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
    assert_eq!(commit_response["commit_text"], Value::String("#$%".to_owned()));
    assert_eq!(commit_response["consumed"], Value::Bool(true));

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
    let (child, mut stdin, mut stdout) = spawn_bridge();

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
fn bridge_consumes_up_down_during_segmented_selection() {
    let (child, mut stdin, mut stdout) = spawn_bridge();

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
