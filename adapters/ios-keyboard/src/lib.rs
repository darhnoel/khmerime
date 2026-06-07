//! iOS custom-keyboard adapter scaffold.
//!
//! The Swift keyboard extension (`UIInputViewController`) owns the QWERTY UI
//! and candidate strip. It calls into `KhmerIMESession` — the UniFFI-exported
//! handle over `khmerime_session::ImeSession` — for every key tap and
//! lifecycle event, then renders the returned `IosRenderState`.
//!
//! References:
//! - UIInputViewController: <https://developer.apple.com/documentation/uikit/uiinputviewcontroller>
//! - UITextDocumentProxy: <https://developer.apple.com/documentation/uikit/uitextdocumentproxy>
//! - Custom Keyboard Guide:
//!   <https://developer.apple.com/library/archive/documentation/General/Conceptual/ExtensibilityPG/CustomKeyboard.html>

use khmerime_session::{NativeKeyEvent, SegmentPreviewEntry, SessionResult, SessionSnapshot};

/// iOS-native key event produced by the Swift keyboard extension.
///
/// All variants convert to `NativeKeyEvent { keycode: 0, state: 0, keyval }`
/// before reaching `ImeSession`. `keycode: 0` is the documented fallback path
/// when no physical scancode is available; the session uses `keyval` directly.
///
/// `Left` and `Right` are not keyboard buttons. The adapter fires them
/// internally when the user taps a segment in the expanded panel: it reads
/// `focused_segment_index` from the last snapshot, computes the delta to the
/// tapped segment index, and fires Left/Right that many times.
///
/// `Tab` and `Escape` (Segment Edit Mode) are deferred until that UI is added.
#[derive(Clone, Debug, PartialEq)]
pub enum IosKeyEvent {
    /// Roman letter tapped on the QWERTY rows.
    Character(char),
    /// ⌫ button — delete one roman char or dissolve a segment.
    Backspace,
    /// Space bar — cycle candidate or insert space.
    Space,
    /// Return button — commit the selected candidate. Always explicit; never fired implicitly.
    Enter,
    /// Fired by the adapter when the user taps a segment to the left of the focused one.
    Left,
    /// Fired by the adapter when the user taps a segment to the right of the focused one.
    Right,
    /// Fired by the adapter when the user taps a candidate at position `n` (1-indexed).
    /// Selects that candidate for the focused segment. Does NOT commit — Enter commits.
    Digit(u8),
}

/// XKB-style keyval constants used by the session contract.
pub const SESSION_KEY_BACKSPACE: u32 = 0xFF08;
pub const SESSION_KEY_RETURN: u32 = 0xFF0D;
pub const SESSION_KEY_SPACE: u32 = 0x20;
pub const SESSION_KEY_LEFT: u32 = 0xFF51;
pub const SESSION_KEY_RIGHT: u32 = 0xFF53;

/// Converts an iOS key event to the session's `NativeKeyEvent`.
///
/// `keycode` is always 0 — software keyboards have no physical scancode.
/// The session falls back to `keyval` when `keycode` is 0.
pub fn ios_key_to_native(event: &IosKeyEvent) -> NativeKeyEvent {
    let keyval = match event {
        IosKeyEvent::Character(ch) => *ch as u32,
        IosKeyEvent::Backspace => SESSION_KEY_BACKSPACE,
        IosKeyEvent::Space => SESSION_KEY_SPACE,
        IosKeyEvent::Enter => SESSION_KEY_RETURN,
        IosKeyEvent::Left => SESSION_KEY_LEFT,
        IosKeyEvent::Right => SESSION_KEY_RIGHT,
        IosKeyEvent::Digit(n) => b'0' as u32 + *n as u32,
    };
    NativeKeyEvent {
        keyval,
        keycode: 0,
        state: 0,
    }
}

/// One entry in the expanded segment panel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IosSegmentEntry {
    /// Khmer output for this segment using the currently selected candidate.
    pub output: String,
    /// Roman input slice represented by this segment.
    pub input: String,
    /// Whether this segment currently owns candidate focus.
    pub focused: bool,
}

impl From<&SegmentPreviewEntry> for IosSegmentEntry {
    fn from(s: &SegmentPreviewEntry) -> Self {
        IosSegmentEntry {
            output: s.output.clone(),
            input: s.input.clone(),
            focused: s.focused,
        }
    }
}

/// Render state returned to the Swift keyboard extension after every session call.
///
/// Swift treats this as the single source of truth for each render cycle:
/// - `candidates` and `selected_index` drive the horizontal candidate strip.
/// - `segments` and `focused_segment_index` drive the expanded segment panel.
/// - `preedit` is shown in the host text field while composing.
/// - `commit_text` is non-None only after Enter; Swift calls
///   `textDocumentProxy.insertText` once and clears it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IosRenderState {
    /// Khmer candidates for the focused segment, shown in the horizontal strip.
    pub candidates: Vec<String>,
    /// Index of the currently selected candidate, if any.
    pub selected_index: Option<usize>,
    /// Roman preedit shown in the host text field while composing.
    pub preedit: String,
    /// Segment entries for the expanded panel. Empty when no segmented session is active.
    pub segments: Vec<IosSegmentEntry>,
    /// Which segment currently owns candidate focus.
    pub focused_segment_index: Option<usize>,
    /// Commit text to push via `textDocumentProxy.insertText`.
    /// None at all times except immediately after Enter is pressed.
    pub commit_text: Option<String>,
}

/// Swift-visible session handle, exported via UniFFI.
///
/// `KeyboardViewController` owns one instance for its lifetime.
/// Each method returns a fresh `IosRenderState` that Swift uses to
/// redraw the candidate strip and segment panel.
///
/// This is a placeholder — wired to `ImeSession` when UniFFI scaffolding lands.
pub struct KhmerIMESession;

impl KhmerIMESession {
    pub fn new() -> Self {
        KhmerIMESession
    }

    /// Call from `viewDidAppear`.
    pub fn focus_in(&self) -> IosRenderState {
        IosRenderState::default()
    }

    /// Call from `viewWillDisappear`.
    pub fn focus_out(&self) -> IosRenderState {
        IosRenderState::default()
    }

    /// Call for every key tap or adapter-internal Left/Right segment navigation event.
    pub fn process_key(&self, event: IosKeyEvent) -> IosRenderState {
        let _ = ios_key_to_native(&event);
        IosRenderState::default()
    }

    /// Call from `selectionDidChange` to anchor the candidate strip UI.
    pub fn set_cursor_location(&self, x: i32, y: i32, width: i32, height: i32) -> IosRenderState {
        let _ = (x, y, width, height);
        IosRenderState::default()
    }
}

/// Derives `IosRenderState` from session output.
pub fn derive_render_state(snapshot: &SessionSnapshot, result: &SessionResult) -> IosRenderState {
    IosRenderState {
        candidates: snapshot.candidates.clone(),
        selected_index: snapshot.selected_index,
        preedit: snapshot.preedit.clone(),
        segments: snapshot.segment_preview.iter().map(IosSegmentEntry::from).collect(),
        focused_segment_index: snapshot.focused_segment_index,
        commit_text: result.commit_text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_converts_with_zero_keycode() {
        let ev = ios_key_to_native(&IosKeyEvent::Character('t'));
        assert_eq!(ev.keyval, 't' as u32);
        assert_eq!(ev.keycode, 0);
        assert_eq!(ev.state, 0);
    }

    #[test]
    fn special_keys_map_to_session_constants() {
        assert_eq!(ios_key_to_native(&IosKeyEvent::Backspace).keyval, SESSION_KEY_BACKSPACE);
        assert_eq!(ios_key_to_native(&IosKeyEvent::Enter).keyval, SESSION_KEY_RETURN);
        assert_eq!(ios_key_to_native(&IosKeyEvent::Space).keyval, SESSION_KEY_SPACE);
        assert_eq!(ios_key_to_native(&IosKeyEvent::Left).keyval, SESSION_KEY_LEFT);
        assert_eq!(ios_key_to_native(&IosKeyEvent::Right).keyval, SESSION_KEY_RIGHT);
    }

    #[test]
    fn digit_maps_to_ascii_digit_keyval() {
        assert_eq!(ios_key_to_native(&IosKeyEvent::Digit(1)).keyval, '1' as u32);
        assert_eq!(ios_key_to_native(&IosKeyEvent::Digit(9)).keyval, '9' as u32);
    }

    #[test]
    fn left_and_right_have_zero_keycode_no_modifier() {
        for ev in [IosKeyEvent::Left, IosKeyEvent::Right] {
            let native = ios_key_to_native(&ev);
            assert_eq!(native.keycode, 0, "no physical scancode on a software keyboard");
            assert_eq!(native.state, 0, "no modifier bits on a software keyboard");
        }
    }

    #[test]
    fn digit_does_not_imply_commit() {
        // Digit selects only. Enter is the separate explicit commit gesture.
        // This test documents the invariant; the real enforcement is in ImeSession.
        let ev = ios_key_to_native(&IosKeyEvent::Digit(2));
        assert_eq!(ev.keyval, '2' as u32);
    }
}
