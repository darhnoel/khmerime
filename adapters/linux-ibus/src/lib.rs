use khmerime_session::CursorLocation;
use serde::{Deserialize, Serialize};

pub mod history_store;

pub use history_store::{desktop_history_path, load_desktop_history, save_desktop_history, DesktopHistoryStore};

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum BridgeCommand {
    ProcessKeyEvent { keyval: u32, keycode: u32, state: u32 },
    FocusIn,
    FocusOut,
    Reset,
    Enable,
    Disable,
    SetCursorLocation { x: i32, y: i32, width: i32, height: i32 },
    Snapshot,
    Shutdown,
}

#[derive(Debug, Serialize)]
pub struct BridgeResponse<S> {
    pub ok: bool,
    pub consumed: bool,
    pub commit_text: Option<String>,
    pub history_changed: bool,
    pub snapshot: S,
    pub error: Option<String>,
}

pub fn fallback_empty_snapshot_json(error: impl Into<String>) -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "consumed": false,
        "commit_text": serde_json::Value::Null,
        "history_changed": false,
        "snapshot": {
            "enabled": false,
            "focused": false,
            "preedit": "",
            "raw_preedit": "",
            "candidates": [],
            "selected_index": serde_json::Value::Null,
            "segmented_active": false,
            "focused_segment_index": serde_json::Value::Null,
            "segment_preview": [],
            "cursor_location": CursorLocation::default(),
        },
        "error": error.into()
    })
}
