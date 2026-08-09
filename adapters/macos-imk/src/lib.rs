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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use khmerime_core::{DecoderConfig, DecoderMode, SpanProposalMode, Transliterator};
use khmerime_session::{
    compute_segmented_refinement, CandidateDisplayEntry, ImeSession, ImeSessionOptions, NativeKeyEvent,
    SegmentPreviewEntry, SegmentedPreviewMode, SessionResult, SessionSnapshot,
};

uniffi::setup_scaffolding!("khmerime_macos_imk");

pub fn macos_live_decoder_config() -> DecoderConfig {
    let mut config = DecoderConfig::shadow_interactive();
    // Keystroke budget (ADR-0005). `shadow_interactive` defaults to the 250 ms *refiner*
    // budget; on the live path that lets a long composition block the keypress — measured
    // at ~544 ms for a single keystroke on a 15-character buffer, which is the "stutters
    // when typing fast" report. IBus's live path uses 75 ms; macOS matches it so a slow
    // decode degrades to the cheaper result instead of holding the keystroke.
    config.wfst_max_latency_ms = 75;
    if let Ok(mode) = std::env::var("KHMERIME_DECODER_MODE") {
        let mode = match mode.as_str() {
            "hybrid" => DecoderMode::Hybrid,
            "weighted-span" | "weighted_span" | "wfst" => DecoderMode::Wfst,
            "legacy" => DecoderMode::Legacy,
            "shadow" => DecoderMode::Shadow,
            _ => config.mode,
        };
        config = config.with_mode(mode);
    }
    // Live keystroke path stays deterministic. Only the instant static-test provider may
    // attach here; the model runs on the debounced visible refiner instead.
    if std::env::var("KHMERIME_SPAN_PROPOSALS").ok().as_deref() == Some("static-test") {
        config = config.with_static_span_proposals();
    }
    config
}

// Debounced visible refiner config — runs the heavier model off the keystroke hot path.
fn macos_visible_refiner_config() -> DecoderConfig {
    let mut config = DecoderConfig::shadow_interactive().with_mode(DecoderMode::Hybrid);
    config.wfst_max_latency_ms = 250;
    match std::env::var("KHMERIME_SPAN_PROPOSALS").ok().as_deref() {
        Some("model") => config = config.with_span_proposal_mode(SpanProposalMode::Model),
        Some("static-test") => config = config.with_static_span_proposals(),
        _ => {}
    }
    config
}

/// Whether a span-proposal provider is configured. When false (dev default), the debounced
/// visible refiner is not built at all — the pause does no work, so the seam stays inert until
/// a provider is plugged in.
fn span_provider_active() -> bool {
    matches!(
        std::env::var("KHMERIME_SPAN_PROPOSALS").ok().as_deref(),
        Some("model") | Some("static-test")
    )
}

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
    /// True when a span-proposal provider contributed this candidate. Swift shows a ✦ marker
    /// (ADR-0016 / ADR-0019). Derived by matching this output against the snapshot's phrase
    /// candidates, since the shared CandidateDisplayEntry does not carry provenance.
    pub from_model: bool,
    /// True when the candidate is a real Lexicon word. A model candidate with this false is shown
    /// with a RED ✦ (unverified — may be a valid name/loanword not yet in the Lexicon), never hidden.
    pub lexicon_verified: bool,
}

// Whitespace-stripped key for matching a display candidate to a phrase candidate. Mirrors the
// session's normalized_suggestion_key without widening its public API.
fn provenance_key(item: &str) -> String {
    item.chars().filter(|ch| !ch.is_whitespace()).collect()
}

impl MacosCandidateDisplayEntry {
    // `provenance`: from_model + lexicon_verified keyed by provenance_key of a phrase candidate's
    // text. A plain Lexicon candidate (absent from the map) defaults to (false, true).
    fn from_display(
        entry: &CandidateDisplayEntry,
        provenance: &std::collections::HashMap<String, (bool, bool)>,
    ) -> Self {
        let (from_model, lexicon_verified) = provenance
            .get(&provenance_key(&entry.output))
            .copied()
            .unwrap_or((false, true));
        MacosCandidateDisplayEntry {
            output: entry.output.clone(),
            recommended: entry.recommended,
            roman_hints: entry.roman_hints.clone(),
            from_model,
            lexicon_verified,
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
    /// True when `preedit` differs from the previous call's value.
    /// Swift calls `setMarkedText` only when this is true, avoiding spurious IPC on
    /// non-composing keys while still clearing marked text when preedit goes empty.
    pub preedit_changed: bool,
}

fn render_state(snapshot: &SessionSnapshot, result: &SessionResult, ready: bool) -> MacosRenderState {
    // Provenance the shared CandidateDisplayEntry drops lives on phrase_candidates (ADR-0019):
    // key each by its whitespace-stripped text so display candidates can look up from_model.
    let provenance: std::collections::HashMap<String, (bool, bool)> = snapshot
        .phrase_candidates
        .iter()
        .map(|p| (provenance_key(&p.text), (p.from_model, p.lexicon_verified)))
        .collect();
    MacosRenderState {
        candidates: snapshot.candidates.clone(),
        candidate_display: snapshot
            .candidate_display
            .iter()
            .map(|entry| MacosCandidateDisplayEntry::from_display(entry, &provenance))
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
        preedit_changed: false, // stamped by with_session after comparing prev_preedit
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
    // Bumped on every keystroke. A lock-free refine captures it at snapshot and re-checks at
    // apply, so a refinement made stale by newer typing is dropped instead of clobbering.
    generation: AtomicU64,
    /// Tracks the preedit from the previous call so `with_session` can set
    /// `preedit_changed` without an extra snapshot allocation per key event.
    prev_preedit: Mutex<String>,
}

#[uniffi::export]
impl MacosIMKSession {
    /// Called once when the IMK process starts (or on first `activateServer:`).
    /// Spawns the warmup thread immediately so engines are ready before the user
    /// types. The returned state has `is_ready: false` — Swift can show a brief
    /// loading indicator if desired.
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        install_panic_logger();
        let session = Arc::new(MacosIMKSession {
            inner: Mutex::new(None),
            ready: Arc::new((Mutex::new(false), Condvar::new())),
            generation: AtomicU64::new(0),
            prev_preedit: Mutex::new(String::new()),
        });
        let s = session.clone();
        std::thread::spawn(move || {
            // TODO: load history from ~/Library/Application Support/khmerime/history.tsv
            let history = std::collections::HashMap::new();
            let input_mode = khmerime_session::InputMode::Roman;

            let live = Transliterator::from_default_data_with_config(macos_live_decoder_config())
                .expect("compiled-in lexicon data must be valid");

            // The visible refiner (heavier engine + any provider) runs on the debounced pause.
            // Built ONLY when a provider is configured — otherwise the seam is inert: no refiner,
            // so the pause does no work and dev-default behaves exactly as before the seam.
            let mut builder = ImeSession::builder(live, history)
                .input_mode(input_mode)
                .options(ImeSessionOptions {
                    segmented_preview: SegmentedPreviewMode::Enabled,
                    // macOS opts into ADR-0013 paging. Must stay equal to the panel's
                    // painted row count (CandidatePanel.pageSize) or page-relative digit
                    // selection breaks — `0` selects the tenth row only when page_size is 10.
                    page_size: MACOS_PAGE_SIZE,
                    ..Default::default()
                });
            let provider_on = span_provider_active();
            // Runtime breadcrumb to the sandbox-readable log (os_log is silent for a sandboxed
            // IME). Tells us, on the actual launched app, whether the warmup thread saw the
            // provider armed — the one fact static inspection can't confirm.
            diag_log(&format!(
                "[khmerime-warmup] span_provider_active={} KHMERIME_SPAN_PROPOSALS={:?}",
                provider_on,
                std::env::var("KHMERIME_SPAN_PROPOSALS").ok()
            ));
            if provider_on {
                let visible_refiner = Transliterator::from_default_data_with_config(macos_visible_refiner_config())
                    .expect("compiled-in lexicon data must be valid");
                builder = builder.visible_refiner(visible_refiner);
            }
            let ime = builder.build();

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
        self.with_render(|s| {
            s.focus_in();
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }

    /// `deactivateServer:` — IMK controller lost the text client.
    pub fn deactivate(&self) -> MacosRenderState {
        self.with_render(|s| {
            s.focus_out();
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }

    /// `commitComposition:` / Escape — discard composition without committing.
    pub fn cancel_composition(&self) -> MacosRenderState {
        self.with_render(|s| {
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
        self.generation.fetch_add(1, Ordering::Relaxed); // invalidates any in-flight refine
        let evdev_keycode = keycode_mac_to_evdev(mac_keycode);
        let xkb_state = modifier_flags_to_xkb_state(modifier_flags);

        // PageUp/PageDown jump a whole page on macOS. Translated here rather than in the
        // shared session so IBus and TSF are unaffected. ↑/↓ are NOT page keys — they fall
        // through to the session's one-step candidate cycling (supersedes ADR-0018, which had
        // ↑/↓ page and left mid-list words unreachable by arrow).
        let page_direction = match keyval {
            KEY_PAGE_UP => -1,
            KEY_PAGE_DOWN => 1,
            _ => 0,
        };

        self.with_render(|s| {
            if page_direction != 0 {
                let snapshot = s.snapshot();
                let len = snapshot.candidates.len();
                if len > 0 {
                    let selected = snapshot.selected_index.unwrap_or(0);
                    let target = page_jump_target(selected, len, MACOS_PAGE_SIZE, page_direction);
                    // Reach the target through the session's own cycling, so selection
                    // bookkeeping (selection_touched, segment sync) stays consistent.
                    let steps = (target as isize - selected as isize).rem_euclid(len as isize) as usize;
                    let mut result = SessionResult {
                        consumed: true,
                        ..SessionResult::default()
                    };
                    for _ in 0..steps {
                        result = s.process_native_key_event(key_event(KEY_DOWN, keycode_mac_to_evdev(0), xkb_state));
                    }
                    return render_state(&s.snapshot(), &result, true);
                }
            }
            let result = s.process_native_key_event(key_event(keyval, evdev_keycode, xkb_state));
            render_state(&s.snapshot(), &result, true)
        })
    }

    /// Cursor position changed — update candidate panel anchor.
    pub fn set_cursor_location(&self, x: i32, y: i32, width: i32, height: i32) -> MacosRenderState {
        self.with_render(|s| {
            s.set_cursor_location(x, y, width, height);
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }

    /// Called by the Swift layer when the visible refiner finishes processing.
    /// Inserts the refined candidate at position 0 if `raw_preedit` still matches
    /// the current composition and no candidate has been manually selected.
    /// Ignored (no-op) when a segmented session is already active.
    pub fn refine_composition(&self, raw_preedit: String) -> MacosRenderState {
        self.with_render(|s| {
            s.apply_refined_candidate(&raw_preedit);
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }

    /// Called by the Swift layer when the visible refiner finishes building a
    /// segmented preview (deferred mode). Rebuilds the segmented session if
    /// `raw_preedit` still matches and no candidate has been touched.
    pub fn refresh_segmented_preview(&self, raw_preedit: String) -> MacosRenderState {
        self.refine_off_lock(&raw_preedit)
    }

    /// Debounced visible refine: re-run the segmented preview through the visible refiner
    /// (the model) on the *current* composition. Called by Swift ~200ms after the last
    /// keystroke. The model runs OFF the session lock, so it never blocks keystrokes.
    pub fn refine_visible(&self) -> MacosRenderState {
        let raw = self.with_session(|s| s.snapshot().raw_preedit.clone());
        self.refine_off_lock(&raw)
    }

    /// Input mode toggle (Roman ↔ NIDA). Clears composition.
    pub fn toggle_input_mode(&self) -> MacosRenderState {
        self.with_render(|s| {
            s.toggle_input_mode();
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
    }
}

impl MacosIMKSession {
    /// Waits up to 500 ms for the warmup thread, then calls `f` with the session.
    /// Returns a default (empty) render state if warmup times out — extremely rare.
    fn with_session<T: Default, F>(&self, f: F) -> T
    where
        F: FnOnce(&mut ImeSession) -> T,
    {
        let (lock, cvar) = &*self.ready;
        let ready = lock.lock().unwrap();
        let _ = cvar.wait_timeout_while(ready, Duration::from_millis(500), |r| !*r);

        let mut guard = self.inner.lock().unwrap();
        match guard.as_mut() {
            Some(session) => f(session),
            None => T::default(),
        }
    }

    /// `with_session` for the render-returning entry points, stamping `preedit_changed` by
    /// comparing against the previous call's preedit. Swift calls `setMarkedText` only when this
    /// is true — clearing marked text when the preedit goes empty (the backspace-on-last-char fix)
    /// without spurious IPC on non-composing keys.
    fn with_render<F>(&self, f: F) -> MacosRenderState
    where
        F: FnOnce(&mut ImeSession) -> MacosRenderState,
    {
        let mut state = self.with_session(f);
        let mut prev = self.prev_preedit.lock().unwrap();
        state.preedit_changed = state.preedit != *prev;
        *prev = state.preedit.clone();
        state
    }

    /// Run the model visible refine WITHOUT holding the session lock across the model.
    /// Snapshot inputs under a brief lock, run the model off-lock, then apply under a brief lock —
    /// dropping the result if a newer keystroke (generation bump) arrived while it computed.
    fn refine_off_lock(&self, raw: &str) -> MacosRenderState {
        let my_gen = self.generation.load(Ordering::Relaxed);
        let snapshot = self.with_session(|s| s.refine_inputs(raw));
        let Some((refiner, raw_owned, history)) = snapshot else {
            return self.with_session(|s| render_state(&s.snapshot(), &SessionResult::default(), true));
        };
        // The model time is spent HERE, holding no session lock — keystrokes run unblocked.
        let refinement = compute_segmented_refinement(&refiner, &raw_owned, &history);
        self.with_session(|s| {
            if self.generation.load(Ordering::Relaxed) == my_gen {
                s.apply_segmented_refinement(&raw_owned, refinement);
            }
            render_state(&s.snapshot(), &SessionResult::default(), true)
        })
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

/// Writes any Rust panic to a file before the process dies.
///
/// The release profile is `panic = "abort"`, so a panic kills the input method
/// instantly — IMK relaunches it silently and macOS writes no usable crash report, so
/// the user just sees the keyboard stop working for a moment. This hook makes such a
/// death diagnosable: the message and location land in the log file below.
fn install_panic_logger() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let location = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "<unknown>".to_owned());
            let message = info.to_string();
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(panic_log_path())
            {
                use std::io::Write;
                let _ = writeln!(file, "[khmerime-panic] {location} :: {message}");
            }
            previous(info);
        }));
    });
}

/// Append one diagnostic line to the sandbox-readable log (same file as panics). os_log is
/// silently dropped for a sandboxed input method, so this is how runtime state is surfaced.
fn diag_log(line: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(panic_log_path())
    {
        use std::io::Write;
        let _ = writeln!(file, "{line}");
    }
}

/// Where `install_panic_logger` records panics.
///
/// The input method is sandboxed (required for third-party IMEs), so `$HOME` here is
/// the app's container — the real path is
/// `~/Library/Containers/com.khmerime.inputmethod.KhmerIMEMacOS/Data/Library/Logs/`.
/// Writes outside the container are silently denied, which is why os_log and
/// home-directory files both come up empty when debugging this process.
fn panic_log_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_owned());
    std::path::PathBuf::from(home).join("Library/Logs/khmerime-imk-panic.log")
}

/// Rows the macOS candidate panel paints per page. Single source of truth: it feeds
/// `ImeSessionOptions.page_size` (so digit selection is page-relative, ADR-0013) and
/// the Up/Down page jump (ADR-0018). `CandidatePanel.pageSize` on the Swift side must
/// match this value.
pub const MACOS_PAGE_SIZE: usize = 10;

/// X11 keysyms for the page keys, as delivered by `KeyvalMapping` on the Swift side.
/// PageUp/PageDown drive the whole-page jump (supersedes ADR-0018's ↑/↓ mapping); the
/// arrows themselves fall through to the session's one-step candidate cycling.
const KEY_PAGE_UP: u32 = 0xFF55;
const KEY_PAGE_DOWN: u32 = 0xFF56;
/// The session's one-step Down keysym — used to drive the page jump through the session's
/// own candidate cycling (so selection bookkeeping stays consistent).
const KEY_DOWN: u32 = 0xFF54;

/// The candidate index a page jump lands on (ADR-0018).
///
/// `direction` is +1 for Down (next page) and -1 for Up (previous page). The row
/// within the page is preserved where possible and clamped to the destination page's
/// length, so a short final page (the lone raw roman fallback) is reachable without
/// overshooting. Pages wrap, matching the way Space wraps the selection.
///
/// This lives in the macOS adapter on purpose: the shared session's `handle_up` /
/// `handle_down` stay one-step so IBus and TSF behavior is unchanged.
pub fn page_jump_target(selected: usize, len: usize, page_size: usize, direction: isize) -> usize {
    if len == 0 {
        return 0;
    }
    let page_size = page_size.max(1);
    let page_count = len.div_ceil(page_size);
    let current_page = selected / page_size;
    let row = selected % page_size;

    let next_page = (current_page as isize + direction).rem_euclid(page_count as isize) as usize;
    let page_start = next_page * page_size;
    let page_len = (len - page_start).min(page_size);
    page_start + row.min(page_len - 1)
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
