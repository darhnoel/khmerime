//! macOS InputMethodKit adapter.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Swift host shell  (adapters/macos-imk/swift/)              │
//! │                                                             │
//! │  KhmerInputController : IMKInputController                  │
//! │    activateServer:   → session.activate()                   │
//! │    deactivateServer: → session.deactivate()                 │
//! │    handle(_:client:) → session.handle_event(keyval,kc,mods) │
//! │    commitComposition → session.cancel_composition()         │
//! │                                                             │
//! │  CandidatePanel : NSPanel (non-activating)                  │
//! │    chips row    → MacosRenderState.segments                 │
//! │    candidates   → MacosRenderState.candidates               │
//! │    positioned via firstRectForCharacterRange:               │
//! └────────────────────┬────────────────────────────────────────┘
//!                      │  UniFFI-generated Swift bindings
//! ┌────────────────────▼────────────────────────────────────────┐
//! │  Rust library  (this crate)                                 │
//! │                                                             │
//! │  MacosIMKSession  (uniffi::Object)                          │
//! │    Arc<Mutex<Option<ImeSession>>>  +  background warmup     │
//! │                                                             │
//! │  MacosRenderState  (uniffi::Record)                         │
//! │    mirrors IosRenderState; preedit = raw roman              │
//! │                                                             │
//! │  keycode_mac_to_evdev(mac_keycode: u16) → u32              │
//! │    needed for NIDA mode (physical key position)             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Comparable adapters
//!
//! | Concern          | Linux IBus            | Windows TSF             | iOS keyboard        | macOS IMK (this)       |
//! |------------------|-----------------------|-------------------------|---------------------|------------------------|
//! | Host language    | Python                | Rust (COM via windows-rs)| Swift (UIKit)       | Swift (AppKit + IMK)   |
//! | Rust bridge      | JSON subprocess       | pure Rust DLL           | UniFFI              | UniFFI                 |
//! | Warmup           | Phase A + Phase B     | async full              | synchronous         | async full (ADR-0002)  |
//! | Candidate UI     | IBus system window    | custom COM UI           | custom UIView       | custom NSPanel (ADR-0003)|
//! | NIDA mode        | yes (evdev keycodes)  | yes (VK → evdev table)  | no                  | yes (mac → evdev table)|
//! | Refiners         | yes (visible + commit)| partial                 | no                  | yes (ADR-0002)         |
//!
//! # Build
//!
//! ```bash
//! make platform-build-macos
//! ```
//!
//! Mirrors `platform-build-ios` in the Makefile:
//!   1. `cargo build -p khmerime_macos_imk --target aarch64-apple-darwin --release`
//!   2. `cargo build -p khmerime_macos_imk --target x86_64-apple-darwin --release`
//!   3. `lipo` → universal static lib
//!   4. `uniffi-bindgen` → Swift bindings in `swift/KhmerIMEInputController/Generated/`
//!   5. `xcodebuild -create-xcframework`
//!   6. `xcodegen generate`
//!
//! # Tests
//!
//! ```bash
//! cargo test -p khmerime_macos_imk
//! ```
//!
//! Tests live in `tests/macos_imk_protocol.rs`. They call `MacosIMKSession` directly —
//! no subprocess, no Swift. The 23 non-Phase-A IBus protocol tests are ported here
//! by name (prefixed `session_` instead of `bridge_`), plus macOS-specific tests for
//! the keycode translation table.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use khmerime_core::{DecoderConfig, Transliterator};
use khmerime_session::{
    CandidateDisplayEntry, ImeSession, ImeSessionOptions, NativeKeyEvent, SegmentPreviewEntry, SegmentedPreviewMode,
    SessionResult, SessionSnapshot,
};

uniffi::setup_scaffolding!("khmerime_macos_imk");

// ── Key constants (XKB / X11 keyval space, same as all other adapters) ────────

const KEY_BACKSPACE: u32 = 0xFF08;
const KEY_RETURN: u32 = 0xFF0D;
const KEY_SPACE: u32 = 0x20;
const KEY_LEFT: u32 = 0xFF51;
const KEY_RIGHT: u32 = 0xFF53;
const KEY_UP: u32 = 0xFF52;
const KEY_DOWN: u32 = 0xFF54;
const KEY_TAB: u32 = 0xFF09;
const KEY_ESCAPE: u32 = 0xFF1B;

fn key_event(keyval: u32, keycode: u32, state: u32) -> NativeKeyEvent {
    NativeKeyEvent { keyval, keycode, state }
}

// ── Public UniFFI types ───────────────────────────────────────────────────────

/// One segment entry — matches IosSegmentEntry field-for-field so both adapters
/// can be compared directly.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct MacosSegmentEntry {
    pub output: String,
    pub input: String,
    pub focused: bool,
}

impl From<&SegmentPreviewEntry> for MacosSegmentEntry {
    fn from(s: &SegmentPreviewEntry) -> Self {
        MacosSegmentEntry {
            output: s.output.clone(),
            input: s.input.clone(),
            focused: s.focused,
        }
    }
}

/// Display metadata for one visible candidate.
///
/// `roman_hints` are exact roman keys for the candidate. If this list is empty,
/// Swift should render the candidate with a derived marker rather than inventing
/// a hint.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct MacosCandidateDisplayEntry {
    pub output: String,
    pub recommended: bool,
    pub roman_hints: Vec<String>,
}

impl From<&CandidateDisplayEntry> for MacosCandidateDisplayEntry {
    fn from(entry: &CandidateDisplayEntry) -> Self {
        MacosCandidateDisplayEntry {
            output: entry.output.clone(),
            recommended: entry.recommended,
            roman_hints: entry.roman_hints.clone(),
        }
    }
}

/// Render state returned to Swift after every session call.
///
/// `preedit` is always the raw roman string (e.g. "khnhomtov") — Swift puts this
/// into marked text via `setMarkedText:selectionRange:`. Khmer is displayed in the
/// `CandidatePanel` via `candidates` and `segments`. See ADR-0003.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct MacosRenderState {
    pub candidates: Vec<String>,
    pub candidate_display: Vec<MacosCandidateDisplayEntry>,
    pub selected_index: Option<u64>,
    /// Raw roman preedit — used as marked text content (see ADR-0003).
    pub preedit: String,
    pub segments: Vec<MacosSegmentEntry>,
    pub focused_segment_index: Option<u64>,
    /// Non-None only immediately after Enter or digit auto-commit.
    /// Swift deletes marked text and inserts this string into the client document.
    pub commit_text: Option<String>,
    pub segment_edit_active: bool,
    pub segment_edit_index: Option<u64>,
    /// False until the background warmup thread finishes loading all three engines.
    pub is_ready: bool,
    /// True when the session consumed the key event (Swift returns true from handle(_:client:)).
    pub consumed: bool,
}

fn render_state(snapshot: &SessionSnapshot, result: &SessionResult, ready: bool) -> MacosRenderState {
    MacosRenderState {
        candidates: snapshot.candidates.clone(),
        candidate_display: snapshot
            .candidate_display
            .iter()
            .map(MacosCandidateDisplayEntry::from)
            .collect(),
        selected_index: snapshot.selected_index.map(|i| i as u64),
        preedit: snapshot.raw_preedit.clone(),
        segments: snapshot.segment_preview.iter().map(MacosSegmentEntry::from).collect(),
        focused_segment_index: snapshot.focused_segment_index.map(|i| i as u64),
        commit_text: result.commit_text.clone(),
        segment_edit_active: snapshot.segment_edit_active,
        segment_edit_index: snapshot.segment_edit_index.map(|i| i as u64),
        is_ready: ready,
        consumed: result.consumed,
    }
}

// ── Session handle ────────────────────────────────────────────────────────────

/// Swift-visible session handle.
///
/// The session is `None` until the background warmup thread finishes loading
/// live + visible refiner + commit refiner engines. On the first key event
/// the calling thread blocks (up to 500 ms) for warmup to complete, then
/// proceeds. In practice on Apple Silicon the full load is ~400 ms; contention
/// is almost never observed. See ADR-0002.
#[derive(uniffi::Object)]
pub struct MacosIMKSession {
    inner: Mutex<Option<ImeSession>>,
    ready: Arc<(Mutex<bool>, Condvar)>,
}

#[uniffi::export]
impl MacosIMKSession {
    /// Called once when the IMK process starts (or on first `activateServer:`).
    /// Spawns the warmup thread immediately so engines are ready before the user
    /// types. The returned state has `is_ready: false` — Swift can show a brief
    /// loading indicator if desired.
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        let session = Arc::new(MacosIMKSession {
            inner: Mutex::new(None),
            ready: Arc::new((Mutex::new(false), Condvar::new())),
        });
        let s = session.clone();
        std::thread::spawn(move || {
            // TODO: load history from ~/Library/Application Support/khmerime/history.tsv
            let history = std::collections::HashMap::new();
            let input_mode = khmerime_session::InputMode::Roman;

            let live = Transliterator::from_default_data_with_config(DecoderConfig::shadow_interactive())
                .expect("compiled-in lexicon data must be valid");

            // TODO: wire visible and commit refiners once ImeSession refiner
            // constructor is available (mirrors IBus bridge Phase B).
            // let visible_refiner = ...;
            // let commit_refiner  = ...;

            let ime = ImeSession::new_with_input_mode_and_options(
                live,
                history,
                input_mode,
                ImeSessionOptions {
                    segmented_preview: SegmentedPreviewMode::Enabled,
                },
            );

            *s.inner.lock().unwrap() = Some(ime);
            let (lock, cvar) = &*s.ready;
            *lock.lock().unwrap() = true;
            cvar.notify_all();
        });
        session
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    /// `activateServer:` — IMK controller became active for a text client.
    pub fn activate(&self) -> MacosRenderState {
        self.with_session(|s| {
            s.focus_in();
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }

    /// `deactivateServer:` — IMK controller lost the text client.
    pub fn deactivate(&self) -> MacosRenderState {
        self.with_session(|s| {
            s.focus_out();
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }

    /// `commitComposition:` / Escape — discard composition without committing.
    pub fn cancel_composition(&self) -> MacosRenderState {
        self.with_session(|s| {
            s.reset();
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }

    // ── Key events ────────────────────────────────────────────────────────────

    /// `handle(_:client:)` — the main key-processing path.
    ///
    /// `keyval` is the Unicode scalar of the key (or an XKB special key constant).
    /// `mac_keycode` is `NSEvent.keyCode` — converted to an evdev scancode for
    /// NIDA mode via `keycode_mac_to_evdev`. `modifier_flags` is the raw
    /// `NSEvent.modifierFlags.rawValue` mapped to XKB-style modifier bits.
    pub fn handle_event(&self, keyval: u32, mac_keycode: u16, modifier_flags: u32) -> MacosRenderState {
        let evdev_keycode = keycode_mac_to_evdev(mac_keycode);
        let xkb_state = modifier_flags_to_xkb_state(modifier_flags);
        self.with_session(|s| {
            let result = s.process_native_key_event(key_event(keyval, evdev_keycode, xkb_state));
            render_state(&s.snapshot(), &result, true)
        })
    }

    /// Cursor position changed — update candidate panel anchor.
    pub fn set_cursor_location(&self, x: i32, y: i32, width: i32, height: i32) -> MacosRenderState {
        self.with_session(|s| {
            s.set_cursor_location(x, y, width, height);
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }

    /// Called by the Swift layer when the visible refiner finishes processing.
    /// Inserts the refined candidate at position 0 if `raw_preedit` still matches
    /// the current composition and no candidate has been manually selected.
    /// Ignored (no-op) when a segmented session is already active.
    pub fn refine_composition(&self, raw_preedit: String) -> MacosRenderState {
        self.with_session(|s| {
            s.apply_refined_candidate(&raw_preedit);
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }

    /// Called by the Swift layer when the visible refiner finishes building a
    /// segmented preview (deferred mode). Rebuilds the segmented session if
    /// `raw_preedit` still matches and no candidate has been touched.
    pub fn refresh_segmented_preview(&self, raw_preedit: String) -> MacosRenderState {
        self.with_session(|s| {
            s.refresh_segmented_preview(&raw_preedit);
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }

    /// Input mode toggle (Roman ↔ NIDA). Clears composition.
    pub fn toggle_input_mode(&self) -> MacosRenderState {
        self.with_session(|s| {
            s.toggle_input_mode();
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }
}

impl MacosIMKSession {
    /// Waits up to 500 ms for the warmup thread, then calls `f` with the session.
    /// Returns a default (empty) render state if warmup times out — extremely rare.
    fn with_session<F>(&self, f: F) -> MacosRenderState
    where
        F: FnOnce(&mut ImeSession) -> MacosRenderState,
    {
        let (lock, cvar) = &*self.ready;
        let ready = lock.lock().unwrap();
        let _ = cvar.wait_timeout_while(ready, Duration::from_millis(500), |r| !*r);

        let mut guard = self.inner.lock().unwrap();
        match guard.as_mut() {
            Some(session) => f(session),
            None => MacosRenderState::default(),
        }
    }
}

// ── NIDA keycode translation ──────────────────────────────────────────────────

/// Converts a macOS virtual keycode (`NSEvent.keyCode`) to an evdev scancode.
///
/// NIDA input mode uses physical key position (evdev keycode) rather than the
/// character value (keyval) to look up the NIDA keymap. This is the same
/// requirement that drives `key_convert.rs` in the Windows TSF adapter.
///
/// The table covers the standard ANSI keyboard layout used by the NIDA XML spec.
/// Keys not in the table return 0 (session ignores keycode=0 and falls back to keyval).
///
/// Reference: evdev keycodes from `linux/input-event-codes.h`, mac keycodes from
/// `Carbon/Events.h` (kVK_* constants).
pub fn keycode_mac_to_evdev(mac_keycode: u16) -> u32 {
    // TODO: fill the full table. Subset shown for illustration; the IBus adapter
    // uses evdev directly (no translation needed on Linux). The Windows adapter
    // has an analogous table in key_convert.rs.
    //
    // mac keycode → evdev scancode
    // (kVK_ANSI_Q=12 → KEY_Q=16, kVK_ANSI_W=13 → KEY_W=17, …)
    match mac_keycode {
        0x00 => 30, // kVK_ANSI_A → KEY_A
        0x01 => 31, // kVK_ANSI_S → KEY_S
        0x02 => 32, // kVK_ANSI_D → KEY_D
        0x03 => 33, // kVK_ANSI_F → KEY_F
        0x04 => 35, // kVK_ANSI_H → KEY_H
        0x05 => 34, // kVK_ANSI_G → KEY_G
        0x06 => 44, // kVK_ANSI_Z → KEY_Z
        0x07 => 45, // kVK_ANSI_X → KEY_X
        0x08 => 46, // kVK_ANSI_C → KEY_C
        0x09 => 47, // kVK_ANSI_V → KEY_V
        0x0B => 48, // kVK_ANSI_B → KEY_B
        0x0C => 16, // kVK_ANSI_Q → KEY_Q
        0x0D => 17, // kVK_ANSI_W → KEY_W
        0x0E => 18, // kVK_ANSI_E → KEY_E
        0x0F => 19, // kVK_ANSI_R → KEY_R
        0x10 => 21, // kVK_ANSI_Y → KEY_Y
        0x11 => 20, // kVK_ANSI_T → KEY_T
        0x12 => 2,  // kVK_ANSI_1 → KEY_1
        0x13 => 3,  // kVK_ANSI_2 → KEY_2
        0x14 => 4,  // kVK_ANSI_3 → KEY_3
        0x15 => 5,  // kVK_ANSI_4 → KEY_4
        0x16 => 7,  // kVK_ANSI_6 → KEY_6
        0x17 => 6,  // kVK_ANSI_5 → KEY_5
        0x18 => 13, // kVK_ANSI_Equal → KEY_EQUAL
        0x19 => 10, // kVK_ANSI_9 → KEY_9
        0x1A => 8,  // kVK_ANSI_7 → KEY_7
        0x1B => 12, // kVK_ANSI_Minus → KEY_MINUS
        0x1C => 9,  // kVK_ANSI_8 → KEY_8
        0x1D => 11, // kVK_ANSI_0 → KEY_0
        0x1E => 27, // kVK_ANSI_RightBracket → KEY_RIGHTBRACE
        0x1F => 24, // kVK_ANSI_O → KEY_O
        0x20 => 22, // kVK_ANSI_U → KEY_U
        0x21 => 26, // kVK_ANSI_LeftBracket → KEY_LEFTBRACE
        0x22 => 23, // kVK_ANSI_I → KEY_I
        0x23 => 25, // kVK_ANSI_P → KEY_P
        0x25 => 38, // kVK_ANSI_L → KEY_L
        0x26 => 36, // kVK_ANSI_J → KEY_J
        0x27 => 40, // kVK_ANSI_Quote → KEY_APOSTROPHE
        0x28 => 37, // kVK_ANSI_K → KEY_K
        0x29 => 39, // kVK_ANSI_Semicolon → KEY_SEMICOLON
        0x2A => 43, // kVK_ANSI_Backslash → KEY_BACKSLASH
        0x2B => 51, // kVK_ANSI_Comma → KEY_COMMA
        0x2C => 53, // kVK_ANSI_Slash → KEY_SLASH
        0x2D => 49, // kVK_ANSI_N → KEY_N
        0x2E => 50, // kVK_ANSI_M → KEY_M
        0x2F => 52, // kVK_ANSI_Period → KEY_DOT
        0x32 => 41, // kVK_ANSI_Grave → KEY_GRAVE
        _ => 0,
    }
}

/// Maps `NSEvent.modifierFlags` bits to XKB-style modifier state bits.
/// Only Shift (bit 0x1) matters for NIDA mode; others are passed for completeness.
pub fn modifier_flags_to_xkb_state(flags: u32) -> u32 {
    const NS_SHIFT_KEY_MASK: u32 = 1 << 17;
    const NS_CONTROL_KEY_MASK: u32 = 1 << 18;
    const NS_ALTERNATE_KEY_MASK: u32 = 1 << 19;

    let mut xkb = 0u32;
    if flags & NS_SHIFT_KEY_MASK != 0 {
        xkb |= 1;
    } // ShiftMask
    if flags & NS_CONTROL_KEY_MASK != 0 {
        xkb |= 4;
    } // ControlMask
    if flags & NS_ALTERNATE_KEY_MASK != 0 {
        xkb |= 8;
    } // Mod1Mask (Alt)
    xkb
}
