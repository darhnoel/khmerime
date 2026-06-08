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

use khmerime_macos_imk::{keycode_mac_to_evdev, MacosIMKSession};

fn session() -> std::sync::Arc<MacosIMKSession> {
    let s = MacosIMKSession::new();
    s.activate();
    s
}

// ── Input mode ────────────────────────────────────────────────────────────────

#[test]
fn session_defaults_to_roman_input_mode() {
    // IBus: bridge_defaults_to_roman_input_mode
    // After activate, the session should be in Roman mode (not NIDA).
    // Type a roman letter — preedit should grow, no direct Khmer commit.
    todo!("port from ibus_bridge_protocol.rs: bridge_defaults_to_roman_input_mode")
}

#[test]
fn session_toggle_input_mode_clears_composition() {
    // IBus: bridge_toggle_input_mode_clears_composition
    todo!()
}

// ── NIDA mode ─────────────────────────────────────────────────────────────────

#[test]
fn session_nida_mode_commits_direct_khmer_key() {
    // IBus: bridge_nida_mode_commits_direct_khmer_key
    // In NIDA mode, a physical key press commits the mapped Khmer character
    // immediately without building a composition.
    todo!()
}

#[test]
fn session_nida_mode_does_not_treat_caps_uppercase_as_shift() {
    // IBus: bridge_nida_mode_does_not_treat_caps_uppercase_as_shift
    todo!()
}

#[test]
fn session_nida_mode_uses_nida_xml_shift_space_mapping() {
    // IBus: bridge_nida_mode_uses_nida_xml_shift_space_mapping
    todo!()
}

#[test]
fn session_nida_mode_uses_evdev_top_letter_row() {
    // IBus: bridge_nida_mode_uses_evdev_top_letter_row
    // Uses mac_keycode → evdev translation (keycode_mac_to_evdev) so physical
    // key position is correct regardless of keyval.
    todo!()
}

#[test]
fn session_nida_mode_does_not_map_backspace_or_enter_evdev_keycodes() {
    // IBus: bridge_nida_mode_does_not_map_backspace_or_enter_evdev_keycodes
    todo!()
}

// ── Basic composition ─────────────────────────────────────────────────────────

#[test]
fn session_commits_raw_roman_when_no_candidate() {
    // IBus: bridge_commits_raw_roman_when_no_candidate
    todo!()
}

#[test]
fn session_commits_single_keycap_digit_immediately() {
    // IBus: bridge_commits_single_keycap_digit_immediately
    // Digit outside composition → Khmer numeral auto-commit (no Enter needed).
    todo!()
}

#[test]
fn session_exposes_candidate_display_metadata() {
    // IBus: bridge_exposes_candidate_display_metadata
    todo!()
}

// ── Cursor tracking ───────────────────────────────────────────────────────────

#[test]
fn session_tracks_cursor_location() {
    // IBus: bridge_tracks_cursor_location_callback
    // CandidatePanel uses this to anchor the NSPanel below the cursor.
    todo!()
}

// ── Segmented sessions ────────────────────────────────────────────────────────

#[test]
fn session_supports_segment_focus_and_full_phrase_commit() {
    // IBus: bridge_supports_segment_focus_and_full_phrase_commit
    todo!()
}

#[test]
fn session_snapshot_exposes_segment_edit_mode_fields() {
    // IBus: bridge_snapshot_exposes_segment_edit_mode_fields
    todo!()
}

#[test]
fn session_consumes_up_down_during_segmented_selection() {
    // IBus: bridge_consumes_up_down_during_segmented_selection
    todo!()
}

#[test]
fn session_commits_live_segmented_long_phrase_on_enter() {
    // IBus: bridge_commits_live_segmented_long_phrase_on_enter
    todo!()
}

// ── Refinement ────────────────────────────────────────────────────────────────

#[test]
fn session_refines_long_phrase_on_enter() {
    // IBus: bridge_refines_long_phrase_on_enter
    // Requires commit refiner to be wired in background warmup (ADR-0002).
    todo!()
}

#[test]
fn session_enter_commits_visible_default_when_hidden_refinement_disagrees() {
    // IBus: bridge_enter_commits_visible_default_when_hidden_refinement_disagrees
    todo!()
}

#[test]
fn session_deferred_visible_refinement_updates_long_phrase_candidate() {
    // IBus: bridge_deferred_visible_refinement_updates_long_phrase_candidate
    todo!()
}

#[test]
fn session_deferred_preview_builds_synchronously_for_digit_selection() {
    // IBus: bridge_deferred_preview_builds_synchronously_for_digit_selection
    todo!()
}

#[test]
fn session_deferred_preview_does_not_revert_user_segment_selection_on_refresh() {
    // IBus: bridge_deferred_preview_does_not_revert_user_segment_selection_on_refresh
    todo!()
}

#[test]
fn session_ignores_stale_visible_refinement_request() {
    // IBus: bridge_ignores_stale_visible_refinement_request
    todo!()
}

#[test]
fn session_refinement_keeps_live_segmented_long_phrase_state() {
    // IBus: bridge_refinement_keeps_live_segmented_long_phrase_state
    todo!()
}

#[test]
fn session_refinement_preserves_segment_focus() {
    // IBus: bridge_refinement_preserves_segment_focus
    todo!()
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
        0x2D, 0x2E,                                       // N M
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
    todo!()
}

#[test]
fn session_segment_entries_populate_render_state() {
    // After typing a two-word composition, MacosRenderState.segments must be
    // non-empty with non-empty input and output on each entry.
    // CandidatePanel uses this to render the chips row.
    todo!()
}
