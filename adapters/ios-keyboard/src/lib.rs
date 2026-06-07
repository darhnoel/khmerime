//! iOS custom-keyboard adapter.
//!
//! `KhmerIMESession` wraps `khmerime_session::ImeSession` behind a UniFFI-exported
//! Arc<Mutex<_>> handle. Swift calls one method per key tap; each returns a fresh
//! `IosRenderState` that drives the strip + candidate panel.

use std::sync::{Arc, Mutex};

use khmerime_core::Transliterator;
use khmerime_session::{ImeSession, NativeKeyEvent, SegmentPreviewEntry, SessionResult, SessionSnapshot};

uniffi::setup_scaffolding!("khmerime_ios_keyboard");

// ── Key constants ────────────────────────────────────────────────────────────

const KEY_BACKSPACE: u32 = 0xFF08;
const KEY_RETURN:    u32 = 0xFF0D;
const KEY_SPACE:     u32 = 0x20;
const KEY_LEFT:      u32 = 0xFF51;
const KEY_RIGHT:     u32 = 0xFF53;

fn key_event(keyval: u32) -> NativeKeyEvent {
    NativeKeyEvent { keyval, keycode: 0, state: 0 }
}

// ── Public UniFFI types ───────────────────────────────────────────────────────

/// One entry in the expanded segment panel / phrase bar.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct IosSegmentEntry {
    pub output: String,
    pub input: String,
    pub focused: bool,
}

impl From<&SegmentPreviewEntry> for IosSegmentEntry {
    fn from(s: &SegmentPreviewEntry) -> Self {
        IosSegmentEntry { output: s.output.clone(), input: s.input.clone(), focused: s.focused }
    }
}

/// Render state returned to Swift after every session call.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct IosRenderState {
    /// Khmer candidates for the focused segment.
    pub candidates: Vec<String>,
    /// Currently selected candidate index, if any.
    pub selected_index: Option<u64>,
    /// Raw roman preedit (what the user typed).
    pub preedit: String,
    /// Segment entries for the phrase bar (empty when no segmented session).
    pub segments: Vec<IosSegmentEntry>,
    /// Which segment owns candidate focus.
    pub focused_segment_index: Option<u64>,
    /// Non-None only immediately after Enter; Swift deletes roman buffer and
    /// inserts this concatenated Khmer string.
    pub commit_text: Option<String>,
}

fn render_state(snapshot: &SessionSnapshot, result: &SessionResult) -> IosRenderState {
    IosRenderState {
        candidates: snapshot.candidates.clone(),
        selected_index: snapshot.selected_index.map(|i| i as u64),
        preedit: snapshot.preedit.clone(),
        segments: snapshot.segment_preview.iter().map(IosSegmentEntry::from).collect(),
        focused_segment_index: snapshot.focused_segment_index.map(|i| i as u64),
        commit_text: result.commit_text.clone(),
    }
}

// ── Session handle ────────────────────────────────────────────────────────────

/// Swift-visible session handle, exported via UniFFI.
///
/// Wraps the real `ImeSession` so every key tap goes through the full
/// romanization → segmentation → candidate ranking pipeline.
#[derive(uniffi::Object)]
pub struct KhmerIMESession {
    inner: Mutex<ImeSession>,
}

#[uniffi::export]
impl KhmerIMESession {
    /// Called once in `viewDidLoad`. Builds the transliterator from compiled-in
    /// data (no external files needed on iOS).
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        let transliterator = Transliterator::from_default_data()
            .expect("compiled-in lexicon data must be valid");
        let session = ImeSession::new(transliterator, std::collections::HashMap::new());
        Arc::new(KhmerIMESession { inner: Mutex::new(session) })
    }

    pub fn focus_in(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        s.focus_in();
        render_state(&s.snapshot(), &SessionResult::default())
    }

    pub fn focus_out(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        s.focus_out();
        render_state(&s.snapshot(), &SessionResult::default())
    }

    pub fn process_character(&self, ch: String) -> IosRenderState {
        let keyval = ch.chars().next().unwrap_or('?') as u32;
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(keyval));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_backspace(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_BACKSPACE));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_space(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_SPACE));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_enter(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_RETURN));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_left(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_LEFT));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_right(&self) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(KEY_RIGHT));
        render_state(&s.snapshot(), &result)
    }

    pub fn process_digit(&self, n: u8) -> IosRenderState {
        let keyval = b'0' as u32 + n as u32;
        let mut s = self.inner.lock().unwrap();
        let result = s.process_native_key_event(key_event(keyval));
        render_state(&s.snapshot(), &result)
    }

    pub fn set_cursor_location(&self, x: i32, y: i32, width: i32, height: i32) -> IosRenderState {
        let mut s = self.inner.lock().unwrap();
        s.set_cursor_location(x, y, width, height);
        render_state(&s.snapshot(), &SessionResult::default())
    }
}
