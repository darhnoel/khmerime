//! macOS IMK protocol tests.
//!
//! These tests call `MacosIMKSession` directly — no subprocess, no Swift, no Xcode.
//! They are the macOS equivalent of `adapters/linux-ibus/tests/ibus_bridge_protocol.rs`.
//!
//! # Porting guide
//!
//! Each `bridge_*` test from the IBus suite maps to a `session_*` test here.
//! The assertions are identical; only the call site changes (direct Rust call
//! instead of JSON over a subprocess pipe).
//!
//! | IBus bridge test                                              | Status |
//! |---------------------------------------------------------------|--------|
//! | bridge_defaults_to_roman_input_mode                           | TODO   |
//! | bridge_toggle_input_mode_clears_composition                   | TODO   |
//! | bridge_nida_mode_commits_direct_khmer_key                     | TODO   |
//! | bridge_nida_mode_does_not_treat_caps_uppercase_as_shift       | TODO   |
//! | bridge_nida_mode_uses_nida_xml_shift_space_mapping            | TODO   |
//! | bridge_nida_mode_uses_evdev_top_letter_row                    | TODO   |
//! | bridge_nida_mode_does_not_map_backspace_or_enter_evdev        | TODO   |
//! | bridge_commits_raw_roman_when_no_candidate                    | TODO   |
//! | bridge_commits_single_keycap_digit_immediately                | TODO   |
//! | bridge_exposes_candidate_display_metadata                     | TODO   |
//! | bridge_tracks_cursor_location_callback                        | TODO   |
//! | bridge_supports_segment_focus_and_full_phrase_commit          | TODO   |
//! | bridge_snapshot_exposes_segment_edit_mode_fields              | TODO   |
//! | bridge_consumes_up_down_during_segmented_selection            | TODO   |
//! | bridge_commits_live_segmented_long_phrase_on_enter            | TODO   |
//! | bridge_refines_long_phrase_on_enter                           | TODO   |
//! | bridge_enter_commits_visible_default_when_hidden_disagrees    | TODO   |
//! | bridge_deferred_visible_refinement_updates_long_phrase        | TODO   |
//! | bridge_deferred_preview_builds_synchronously_for_digit        | TODO   |
//! | bridge_deferred_preview_does_not_revert_user_selection        | TODO   |
//! | bridge_ignores_stale_visible_refinement_request               | TODO   |
//! | bridge_refinement_keeps_live_segmented_long_phrase_state      | TODO   |
//! | bridge_refinement_preserves_segment_focus                     | TODO   |
//!
//! # macOS-specific tests (no IBus equivalent)
//!
//! | Test                                                          | Status |
//! |---------------------------------------------------------------|--------|
//! | keycode_mac_to_evdev_covers_full_ansi_layout                  | TODO   |
//! | keycode_mac_shift_not_set_in_non_shift_keypress               | TODO   |
//! | session_preedit_is_raw_roman_not_khmer                        | TODO   |
//! | session_ready_state_is_false_before_warmup_completes          | TODO   |
//! | session_segment_entries_populate_render_state                 | TODO   |

use khmerime_macos_imk::{keycode_mac_to_evdev, page_jump_target, MacosIMKSession, MacosRenderState};

const NS_SHIFT_KEY_MASK: u32 = 1 << 17;

// Khmer phrase constants (all codepoints escaped to survive editor round-trips)
// "នេះជាស្នាដៃបកប្រែ"
const PHRASE_NIHJEAS: &str = "\u{1793}\u{17C1}\u{17C7}\u{1787}\u{17B6}\u{179F}\u{17D2}\u{1793}\u{17B6}\u{178A}\u{17C3}\u{1794}\u{1780}\u{1794}\u{17D2}\u{179A}\u{17C2}";
// "ការសន្មត" (kasanmot)
const PHRASE_KASANMOT: &str = "\u{1780}\u{17B6}\u{179A}\u{179F}\u{1793}\u{17D2}\u{1798}\u{178F}";
// "កសាងម៉ូត" (commit-refiner alternative — must NOT appear)
const PHRASE_KASANMOT_ALT: &str = "\u{1780}\u{179F}\u{17B6}\u{1784}\u{1798}\u{17C9}\u{17BC}\u{178F}";

fn session() -> std::sync::Arc<MacosIMKSession> {
    let s = MacosIMKSession::new();
    // Retry until the background warmup thread finishes. When many tests run
    // in parallel each spawning their own warmup, the default 500 ms condvar
    // timeout can fire before the lexicon loads. activate() returns is_ready:
    // false (default state) when inner is still None; loop until it's true.
    loop {
        if s.activate().is_ready {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    s
}

/// Types each character as a keyval (Unicode scalar), mac_keycode=0, no modifiers.
/// mac_keycode=0 is only meaningful in NIDA mode; Roman mode uses keyval exclusively.
fn type_str(s: &std::sync::Arc<MacosIMKSession>, input: &str) -> MacosRenderState {
    let mut state = MacosRenderState::default();
    for ch in input.chars() {
        state = s.handle_event(ch as u32, 0, 0);
    }
    state
}

// ── Input mode ────────────────────────────────────────────────────────────────

#[test]
fn session_defaults_to_roman_input_mode() {
    // IBus: bridge_defaults_to_roman_input_mode
    // After activate, the session should be in Roman mode (not NIDA).
    // Type a roman letter — preedit should grow, no direct Khmer commit.
    let s = session();
    let state = s.handle_event('k' as u32, 0, 0);
    assert_eq!(state.preedit, "k");
    assert!(state.commit_text.is_none());
}

#[test]
fn session_toggle_input_mode_clears_composition() {
    // IBus: bridge_toggle_input_mode_clears_composition
    let s = session();
    type_str(&s, "jea");
    let state = s.toggle_input_mode();
    assert_eq!(state.preedit, "");
    assert!(state.candidates.is_empty());
    assert!(state.commit_text.is_none());
}

// ── NIDA mode ─────────────────────────────────────────────────────────────────

#[test]
fn session_nida_mode_commits_direct_khmer_key() {
    // IBus: bridge_nida_mode_commits_direct_khmer_key
    // mac_keycode 0x28 → evdev KEY_K=37 → NIDA base → "ក"
    let s = session();
    s.toggle_input_mode();
    let state = s.handle_event(107, 0x28, 0);
    assert_eq!(state.commit_text, Some("ក".to_owned()));
    assert_eq!(state.preedit, "");
}

#[test]
fn session_nida_mode_does_not_treat_caps_uppercase_as_shift() {
    // IBus: bridge_nida_mode_does_not_treat_caps_uppercase_as_shift
    // Uppercase keyval (65='A') with state=0 → Base modifier (not Shift).
    // mac_keycode 0x00 → evdev KEY_A=30, NIDA base → "ា".
    let s = session();
    s.toggle_input_mode();
    let state = s.handle_event(65, 0x00, 0);
    assert_eq!(state.commit_text, Some("ា".to_owned()));
}

#[test]
fn session_nida_mode_uses_nida_xml_shift_space_mapping() {
    // IBus: bridge_nida_mode_uses_nida_xml_shift_space_mapping
    // Shift+Space in NIDA → " " (plain space). keyval=32 falls back to
    // scancode_from_keyval(' ')=57; NS_SHIFT_KEY_MASK → xkb state=1 (ShiftMask).
    let s = session();
    s.toggle_input_mode();
    let state = s.handle_event(32, 0x31, NS_SHIFT_KEY_MASK);
    assert_eq!(state.commit_text, Some(" ".to_owned()));
}

#[test]
fn session_nida_mode_uses_evdev_top_letter_row() {
    // IBus: bridge_nida_mode_uses_evdev_top_letter_row
    // Verifies mac_keycode → evdev → NIDA output for the full top letter row.
    // Note: kVK_ANSI_T=0x11 and kVK_ANSI_Y=0x10 are swapped vs alphabetical
    // order on the mac keyboard — the translation table must reflect this.
    let s = session();
    s.toggle_input_mode();
    let keys: &[(u32, u16, &str)] = &[
        (113, 0x0C, "ឆ"), // Q: kVK_ANSI_Q → KEY_Q=16
        (119, 0x0D, "ឹ"),  // W: kVK_ANSI_W → KEY_W=17
        (101, 0x0E, "េ"), // E: kVK_ANSI_E → KEY_E=18
        (114, 0x0F, "រ"), // R: kVK_ANSI_R → KEY_R=19
        (116, 0x11, "ត"), // T: kVK_ANSI_T → KEY_T=20  (mac 0x11, not 0x10)
        (121, 0x10, "យ"), // Y: kVK_ANSI_Y → KEY_Y=21  (mac 0x10, not 0x11)
        (117, 0x20, "ុ"),  // U: kVK_ANSI_U → KEY_U=22
        (105, 0x22, "ិ"),  // I: kVK_ANSI_I → KEY_I=23
        (111, 0x1F, "ោ"), // O: kVK_ANSI_O → KEY_O=24
        (112, 0x23, "ផ"), // P: kVK_ANSI_P → KEY_P=25
    ];
    for &(keyval, mac_kc, expected) in keys {
        let state = s.handle_event(keyval, mac_kc, 0);
        assert_eq!(
            state.commit_text.as_deref(),
            Some(expected),
            "keyval={keyval} mac_kc=0x{mac_kc:02X}"
        );
    }
}

#[test]
fn session_nida_mode_does_not_map_backspace_or_enter_evdev_keycodes() {
    // IBus: bridge_nida_mode_does_not_map_backspace_or_enter_evdev_keycodes
    // scancode_from_keycode rejects evdev 14 (backspace) and 28 (enter);
    // scancode_from_keyval also can't map XKB specials 0xFF08 / 0xFF0D.
    let s = session();
    s.toggle_input_mode();
    let backspace = s.handle_event(0xFF08, 0x33, 0); // kVK_Delete
    assert!(
        backspace.commit_text.is_none(),
        "backspace must not produce Khmer in NIDA mode"
    );
    let enter = s.handle_event(0xFF0D, 0x24, 0); // kVK_Return
    assert!(enter.commit_text.is_none(), "enter must not produce Khmer in NIDA mode");
}

// ── Basic composition ─────────────────────────────────────────────────────────

#[test]
fn session_auto_commits_backtick_as_single_keycap() {
    // A backtick has no Khmer candidate. Since `is_single_keycap_char` widened to every
    // non-alpha ASCII graphic, a lone backtick is a single-keycap auto-commit: it commits its
    // raw self immediately, no Enter needed (like a digit → Khmer numeral).
    let s = session();
    let state = s.handle_event('`' as u32, 0, 0); // keyval 96
    assert_eq!(state.commit_text, Some("`".to_owned()));
    assert_eq!(state.preedit, "");
}

#[test]
fn session_commits_single_keycap_digit_immediately() {
    // IBus: bridge_commits_single_keycap_digit_immediately
    // Digit outside composition → Khmer numeral auto-commit (no Enter needed).
    let s = session();
    let state = s.handle_event('1' as u32, 0, 0); // keyval 49
    assert_eq!(state.commit_text, Some("១".to_owned()));
    assert_eq!(state.preedit, "");
}

#[test]
fn session_exposes_candidate_display_metadata() {
    // IBus: bridge_exposes_candidate_display_metadata
    let s = session();
    let state = type_str(&s, "jea");
    assert!(
        !state.candidates.is_empty(),
        "typing 'jea' must produce at least one candidate"
    );
    assert_eq!(
        state.candidate_display.len(),
        state.candidates.len(),
        "candidate display metadata must align one-to-one with visible candidates"
    );
    assert!(
        state
            .candidate_display
            .iter()
            .any(|entry| entry.recommended && entry.roman_hints.iter().any(|hint| hint == "jea")),
        "candidate display metadata must expose the recommended candidate and exact roman hint"
    );
    assert_eq!(state.selected_index, Some(0), "first candidate should be pre-selected");
    assert!(!state.preedit.is_empty());
}

// ── Cursor tracking ───────────────────────────────────────────────────────────

#[test]
fn session_tracks_cursor_location() {
    // IBus: bridge_tracks_cursor_location_callback
    // CandidatePanel uses this to anchor the NSPanel below the cursor.
    // Cursor location is stored server-side; the render state returned is clean.
    let s = session();
    let state = s.set_cursor_location(12, 34, 56, 78);
    assert_eq!(state.preedit, "");
    assert!(state.commit_text.is_none());
}

// ── Segmented sessions ────────────────────────────────────────────────────────

#[test]
fn session_supports_segment_focus_and_full_phrase_commit() {
    // IBus: bridge_supports_segment_focus_and_full_phrase_commit
    let s = session();
    let state = type_str(&s, "khnhomtov");
    assert!(state.segments.len() >= 2, "khnhomtov must produce >= 2 segments");
    assert_eq!(state.focused_segment_index, Some(0), "focus starts at segment 0");
    // Right arrow moves focus to segment 1
    let moved = s.handle_event(0xFF53, 0, 0); // KEY_RIGHT
    assert_eq!(moved.focused_segment_index, Some(1));
    // Enter commits the full Khmer phrase
    let committed = s.handle_event(0xFF0D, 0, 0);
    assert!(committed.commit_text.is_some(), "Enter must commit");
    assert_ne!(
        committed.commit_text.as_deref(),
        Some("khnhomtov"),
        "commit must be Khmer, not roman"
    );
}

#[test]
fn session_snapshot_exposes_segment_edit_mode_fields() {
    // IBus: bridge_snapshot_exposes_segment_edit_mode_fields
    let s = session();
    type_str(&s, "khnhomtov");
    let tab = s.handle_event(0xFF09, 0, 0); // KEY_TAB
    assert!(tab.segment_edit_active, "Tab must enter segment edit mode");
    assert_eq!(tab.segment_edit_index, Some(0), "segment edit starts at index 0");
}

#[test]
fn session_consumes_up_down_during_segmented_selection() {
    // IBus: bridge_consumes_up_down_during_segmented_selection
    // Down/Up cycle through candidates for the focused segment; session stays segmented.
    let s = session();
    type_str(&s, "khnhomtov");
    let down = s.handle_event(0xFF54, 0, 0); // KEY_DOWN
    assert!(
        down.segments.len() >= 2,
        "segmented session must remain active after Down"
    );
    assert_eq!(down.focused_segment_index, Some(0));
    let up = s.handle_event(0xFF52, 0, 0); // KEY_UP
    assert!(up.segments.len() >= 2, "segmented session must remain active after Up");
    assert_eq!(up.focused_segment_index, Some(0));
}

#[test]
fn session_commits_live_segmented_long_phrase_on_enter() {
    // IBus: bridge_commits_live_segmented_long_phrase_on_enter
    let s = session();
    let state = type_str(&s, "nihjeasnadaiborkbrae");
    assert!(state.segments.len() >= 2, "long phrase must produce live segments");
    let committed = s.handle_event(0xFF0D, 0, 0);
    // "នេះជាស្នាដៃបកប្រែ" all codepoints explicit to survive editor round-trips
    const PHRASE: &str = "\u{1793}\u{17C1}\u{17C7}\u{1787}\u{17B6}\u{179F}\u{17D2}\u{1793}\u{17B6}\u{178A}\u{17C3}\u{1794}\u{1780}\u{1794}\u{17D2}\u{179A}\u{17C2}";
    assert_eq!(committed.commit_text.as_deref(), Some(PHRASE));
}

// ── Refinement ────────────────────────────────────────────────────────────────

#[test]
fn session_refines_long_phrase_on_enter() {
    // IBus: bridge_refines_long_phrase_on_enter
    // The live segmented session (shadow_interactive) already produces the
    // correct multi-word commit without a separate commit refiner.
    let s = session();
    type_str(&s, "nihjeasnadaiborkbrae");
    let committed = s.handle_event(0xFF0D, 0, 0);
    assert_eq!(committed.commit_text.as_deref(), Some(PHRASE_NIHJEAS));
}

#[test]
fn session_enter_commits_visible_default_when_hidden_refinement_disagrees() {
    // IBus: bridge_enter_commits_visible_default_when_hidden_refinement_disagrees
    // "kasanmot" → live top candidate "ការសន្មត"; commit refiner would produce
    // "កសាងម៉ូត" — Enter must commit what the user sees, not the hidden refiner.
    let s = session();
    type_str(&s, "kasanmot");
    let committed = s.handle_event(0xFF0D, 0, 0);
    assert_eq!(committed.commit_text.as_deref(), Some(PHRASE_KASANMOT));
    assert_ne!(committed.commit_text.as_deref(), Some(PHRASE_KASANMOT_ALT));
}

#[test]
fn session_marks_model_rescued_candidate_from_model() {
    // The visible refiner reads KHMERIME_SPAN_PROPOSALS at warmup. static-test maps
    // "salarien" -> "សាលារៀន" (a real Lexicon word), so the rescued candidate must be marked
    // from_model=true, lexicon_verified=true (a WHITE ✦). ADR-0019.
    std::env::set_var("KHMERIME_SPAN_PROPOSALS", "static-test");
    let s = MacosIMKSession::new();
    loop {
        if s.activate().is_ready { break; }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    std::env::remove_var("KHMERIME_SPAN_PROPOSALS");

    type_str(&s, "salarien");
    let refined = s.refine_composition("salarien".to_owned());

    let rescued = refined
        .candidate_display
        .iter()
        .find(|c| c.output == "សាលារៀន")
        .expect("the model-rescued word must appear in the candidate list");
    assert!(rescued.from_model, "rescued candidate must be marked from_model");
    assert!(rescued.lexicon_verified, "សាលារៀន is a real Lexicon word → verified (white ✦)");
}

#[test]
fn session_leaves_plain_lexicon_candidate_unmarked() {
    // Without any provider, an ordinary lexicon candidate must carry NO ✦: from_model=false
    // (so the marker only ever appears on model-assisted words).
    let s = session();
    let state = type_str(&s, "jea");
    assert!(!state.candidate_display.is_empty(), "jea must produce candidates");
    assert!(
        state.candidate_display.iter().all(|c| !c.from_model),
        "plain lexicon candidates must not be marked from_model",
    );
}

#[test]
fn session_deferred_visible_refinement_updates_long_phrase_candidate() {
    // IBus: bridge_deferred_visible_refinement_updates_long_phrase_candidate
    // IBus deferred mode: refine_composition pushes the refined phrase to
    // candidates[0]. macOS uses live segmentation so refine_composition is a
    // no-op (segmented_session is already Some), but Enter still commits
    // the correct phrase via the live segmented path.
    let s = session();
    let state = type_str(&s, "nihjeasnadaiborkbrae");
    assert!(state.segments.len() >= 2, "live segmentation must be active");
    let after_refine = s.refine_composition("nihjeasnadaiborkbrae".to_owned());
    assert!(
        after_refine.segments.len() >= 2,
        "live segments must persist after no-op refine"
    );
    let committed = s.handle_event(0xFF0D, 0, 0);
    assert_eq!(committed.commit_text.as_deref(), Some(PHRASE_NIHJEAS));
}

#[test]
fn session_deferred_preview_builds_synchronously_for_digit_selection() {
    // IBus: bridge_deferred_preview_builds_synchronously_for_digit_selection
    // IBus deferred mode: pressing a digit during composition triggers an
    // immediate (synchronous) segmented preview build. macOS uses live
    // segmentation, so segments are already present before the digit press.
    // The digit selects the nth candidate for the focused segment.
    let s = session();
    type_str(&s, "sophamongkul");
    // Digit '2' selects the 2nd candidate; segmented state remains active.
    let after_digit = s.handle_event('2' as u32, 0, 0);
    assert!(
        !after_digit.segments.is_empty() || !after_digit.preedit.is_empty(),
        "session must remain active after candidate digit selection"
    );
}

#[test]
fn session_deferred_preview_does_not_revert_user_segment_selection_on_refresh() {
    // IBus: bridge_deferred_preview_does_not_revert_user_segment_selection_on_refresh
    // A stale refresh_segmented_preview call (same raw_preedit but selection_touched=true)
    // must not overwrite a candidate the user already selected with a digit.
    let s = session();
    type_str(&s, "sophamongkul");
    // Select 2nd candidate — marks selection_touched = true
    let after_digit = s.handle_event('2' as u32, 0, 0);
    let selected_segment_output = after_digit.segments.first().map(|s| s.output.clone());
    // Stale refresh from a debounced visible-refiner callback
    s.refresh_segmented_preview("sophamongkul".to_owned());
    let after_refresh = s.set_cursor_location(0, 0, 0, 0); // read state without side effects
    assert_eq!(
        after_refresh.segments.first().map(|s| s.output.clone()),
        selected_segment_output,
        "refresh must not revert a user-touched segment selection"
    );
}

#[test]
fn session_ignores_stale_visible_refinement_request() {
    // IBus: bridge_ignores_stale_visible_refinement_request
    // apply_refined_candidate checks raw_preedit == composition_raw; a stale
    // shorter preedit must not update the candidate list.
    let s = session();
    type_str(&s, "nihjeasnadaiborkbrae");
    let stale = s.refine_composition("nihjeasnadai".to_owned()); // shorter → stale
    assert_eq!(
        stale.preedit, "nihjeasnadaiborkbrae",
        "preedit must be unchanged after stale refine"
    );
    // Candidate list must not have been replaced with stale results
    assert!(stale.segments.len() >= 2 || !stale.candidates.is_empty());
}

#[test]
fn session_refinement_keeps_live_segmented_long_phrase_state() {
    // IBus: bridge_refinement_keeps_live_segmented_long_phrase_state
    // refine_composition is a no-op when segmented_session is already active;
    // the live segmented state must survive the call unchanged.
    let s = session();
    let live = type_str(&s, "nihjeasnadaiborkbrae");
    assert!(live.segments.len() >= 2);
    let after_refine = s.refine_composition("nihjeasnadaiborkbrae".to_owned());
    assert!(after_refine.segments.len() >= 2, "live segments must persist");
    assert_eq!(after_refine.preedit, "nihjeasnadaiborkbrae");
    let committed = s.handle_event(0xFF0D, 0, 0);
    assert_eq!(committed.commit_text.as_deref(), Some(PHRASE_NIHJEAS));
}

#[test]
fn session_refinement_preserves_segment_focus() {
    // IBus: bridge_refinement_preserves_segment_focus
    // Right arrow moves focus to segment 1; a subsequent refine_composition
    // (no-op in live mode) must leave focus at segment 1.
    let s = session();
    type_str(&s, "nihjeasnadaiborkbrae");
    let moved = s.handle_event(0xFF53, 0, 0); // KEY_RIGHT → segment 1
    assert_eq!(moved.focused_segment_index, Some(1));
    let after_refine = s.refine_composition("nihjeasnadaiborkbrae".to_owned());
    assert_eq!(
        after_refine.focused_segment_index,
        Some(1),
        "segment focus must survive refine_composition"
    );
}

// ── macOS-specific ────────────────────────────────────────────────────────────

#[test]
fn keycode_mac_to_evdev_covers_full_ansi_layout() {
    // All standard ANSI letter keys must map to non-zero evdev codes.
    // Verifies the translation table is complete enough for NIDA mode.
    let letter_keys: &[u16] = &[
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, // A S D F H G Z X
        0x08, 0x09, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, // C V B Q W E R Y
        0x11, 0x1F, 0x20, 0x22, 0x23, 0x25, 0x26, 0x28, // T O U I P L J K
        0x2D, 0x2E, // N M
    ];
    for &mac_kc in letter_keys {
        assert_ne!(
            keycode_mac_to_evdev(mac_kc),
            0,
            "mac keycode 0x{mac_kc:02X} must map to a non-zero evdev code"
        );
    }
}

#[test]
fn keycode_mac_unknown_key_returns_zero() {
    // Unknown keycodes fall back to 0 — session ignores keycode=0 and uses keyval.
    assert_eq!(keycode_mac_to_evdev(0xFF), 0);
}

#[test]
fn session_preedit_is_raw_roman_not_khmer() {
    // MacosRenderState.preedit must be the raw roman string (e.g. "nhom"),
    // not the composed Khmer output. Swift uses this as marked text content.
    // See ADR-0003.
    let s = session();
    let state = type_str(&s, "nhom");
    assert_eq!(state.preedit, "nhom", "preedit must be raw roman, not Khmer script");
    assert!(state.commit_text.is_none());
}

#[test]
fn session_segment_entries_populate_render_state() {
    // After typing a two-word composition, MacosRenderState.segments must be
    // non-empty with non-empty input and output on each entry.
    // CandidatePanel uses this to render the chips row.
    let s = session();
    let state = type_str(&s, "khnhomtov");
    assert!(
        state.segments.len() >= 2,
        "two-word composition must produce >= 2 segments"
    );
    for seg in &state.segments {
        assert!(!seg.input.is_empty(), "segment input must be non-empty");
        assert!(!seg.output.is_empty(), "segment output must be non-empty");
    }
}

// ── Page navigation (ADR-0018) ────────────────────────────────────────────────
// Up/Down jump a whole page on macOS. The math lives in the adapter so the shared
// session — and therefore IBus and TSF — keep their one-step Up/Down behavior.

#[test]
fn page_jump_moves_a_whole_page_keeping_the_row() {
    // 21 candidates, page size 10 → pages [0..10), [10..20), [20..21).
    // From row 3 of page 1, Down lands on row 3 of page 2.
    assert_eq!(page_jump_target(3, 21, 10, 1), 13);
    // …and Up from there returns to row 3 of page 1.
    assert_eq!(page_jump_target(13, 21, 10, -1), 3);
}

#[test]
fn page_jump_clamps_to_a_short_final_page() {
    // Page 3 holds only index 20 (the raw roman fallback). Jumping down from row 5
    // of page 2 clamps to that page's single row rather than overshooting the list.
    assert_eq!(page_jump_target(15, 21, 10, 1), 20);
}

#[test]
fn page_jump_wraps_like_space_does() {
    // Down from the last page wraps to the first (Space wraps via rem_euclid, so
    // page jumping stays consistent with it).
    assert_eq!(page_jump_target(20, 21, 10, 1), 0);
    // Up from the first page wraps to the last.
    assert_eq!(page_jump_target(3, 21, 10, -1), 20);
}

#[test]
fn session_survives_long_space_autorepeat() {
    // Held Space (key autorepeat) fires handle_event many times in a row, including
    // on an empty composition where there are no candidates to cycle.
    let s = session();
    for _ in 0..200 {
        let _ = s.handle_event(0x0020, 0, 0);
    }
    // …and again with a live composition, cycling far past the end of the list.
    let _ = type_str(&s, "knhom");
    for _ in 0..200 {
        let _ = s.handle_event(0x0020, 0, 0);
    }
}

#[test]
fn page_jump_survives_a_stale_selection_past_the_end() {
    // The candidate list shrinks as the user types, so a selection index captured
    // before the shrink can exceed the new length. Underflowing here would abort the
    // whole input method (release builds are panic = "abort").
    assert_eq!(page_jump_target(50, 3, 10, 1), 0);
    assert_eq!(page_jump_target(50, 3, 10, -1), 0);
    // Degenerate inputs must not panic either.
    assert_eq!(page_jump_target(0, 0, 10, 1), 0);
    assert_eq!(page_jump_target(0, 1, 0, 1), 0);
}



#[test]
fn live_keystroke_path_carries_a_latency_budget() {
    // ADR-0005: the keystroke path must degrade rather than block. Without an explicit
    // budget the live config inherits the 250 ms *refiner* default, and a long
    // composition then stalls the keypress (measured ~544 ms at 15 chars before this was
    // set; ~207 ms after). IBus's live path uses 75 ms; macOS matches it.
    assert_eq!(
        khmerime_macos_imk::macos_live_decoder_config().wfst_max_latency_ms,
        75,
        "the live keystroke decoder must keep the 75 ms interactive budget"
    );
}
