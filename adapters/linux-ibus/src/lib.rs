//! Linux IBus bridge protocol types and desktop persistence exports.
//!
//! The Linux runtime has two native pieces: a Python IBus adapter that receives
//! desktop callbacks, and a Rust bridge process that owns `khmerime_session`.
//! This crate defines the JSON command/response boundary between them.

use khmerime_session::{CursorLocation, InputMode};
use serde::{Deserialize, Serialize};

pub mod history_store;

pub use history_store::{desktop_history_path, load_desktop_history, save_desktop_history, DesktopHistoryStore};

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum BridgeCommand {
    ProcessKeyEvent { keyval: u32, keycode: u32, state: u32 },
    RefineComposition { raw_preedit: String },
    RefreshSegmentedPreview { raw_preedit: String },
    SetInputMode { input_mode: InputMode },
    ToggleInputMode,
    FocusIn,
    FocusOut,
    Reset,
    Enable,
    Disable,
    SetCursorLocation { x: i32, y: i32, width: i32, height: i32 },
    Snapshot,
    Shutdown,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BridgeReadiness {
    PhaseA,
    FullPending,
    Full,
    Failed,
}

/// Per-phase millisecond timings for a single `ProcessKeyEvent` command, so the
/// Python adapter can attribute bridge-side cost without parsing stderr.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct BridgeTimings {
    pub wait_refiner_ms: f64,
    pub sync_segpreview_ms: f64,
    pub process_event_ms: f64,
    pub history_save_ms: f64,
    pub snapshot_ms: f64,
    pub total_ms: f64,
}

/// JSON response returned by the Rust bridge after every command.
///
/// Python should render `snapshot`, commit `commit_text` once, and use
/// `consumed` to decide whether IBus should swallow the original key event.
#[derive(Debug, Serialize)]
pub struct BridgeResponse<S> {
    pub ok: bool,
    pub consumed: bool,
    pub commit_text: Option<String>,
    pub history_changed: bool,
    pub readiness: BridgeReadiness,
    pub snapshot: S,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timings: Option<BridgeTimings>,
}

pub fn fallback_empty_snapshot_json(error: impl Into<String>) -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "consumed": false,
        "commit_text": serde_json::Value::Null,
        "history_changed": false,
        "readiness": "failed",
        "snapshot": {
            "enabled": false,
            "focused": false,
            "input_mode": "roman",
            "preedit": "",
            "raw_preedit": "",
            "candidates": [],
            "candidate_display": [],
            "selected_index": serde_json::Value::Null,
            "segmented_active": false,
            "focused_segment_index": serde_json::Value::Null,
            "segment_preview": [],
            "cursor_location": CursorLocation::default(),
        },
        "error": error.into()
    })
}
