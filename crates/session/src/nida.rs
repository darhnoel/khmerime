//! NIDA input mode: direct-Khmer keymap entry.
//!
//! `InputMode::Nida` bypasses the roman decoder entirely. Each printable key
//! maps straight to Khmer output (base / shift / AltGr rows) and commits
//! immediately with no Composition or Preedit. This module owns both halves of
//! that mode: the [`ImeSession::process_nida_key_event`] handler and the keymap
//! table it looks up, loaded once from the bundled `nida_keymap.csv`.
//!
//! The keymap is keyed on a *canonical scancode* — the set-1 / evdev numbering
//! shared by the main typing block across platforms. Linux IBus already
//! delivers evdev keycodes that equal these scancodes; other adapters must
//! normalize their native codes into the same space before calling the session
//! (see [`NativeKeyEvent::keycode`](crate::adapter_contract::NativeKeyEvent)).

use std::sync::OnceLock;

use crate::adapter_contract::SessionResult;
use crate::ime_session::{ImeSession, STATE_MOD5_MASK, STATE_SHIFT_MASK};

const NIDA_KEYMAP_CSV: &str = include_str!("../../../data/keymaps/nida_keymap.csv");

impl ImeSession {
    pub(crate) fn process_nida_key_event(&mut self, keyval: u32, keycode: u32, state: u32) -> SessionResult {
        let modifiers = if state & STATE_MOD5_MASK != 0 {
            NidaModifiers::AltGr
        } else if state & STATE_SHIFT_MASK != 0 {
            NidaModifiers::Shift
        } else {
            NidaModifiers::Base
        };
        let Some(output) = lookup_nida_output(keyval, keycode, modifiers) else {
            return SessionResult::default();
        };
        SessionResult {
            consumed: true,
            commit_text: Some(output.to_owned()),
            history_changed: false,
        }
    }
}

#[derive(Debug)]
struct NidaKeymapEntry {
    /// Canonical scancode (set-1 / evdev main block) this row maps from.
    scancode: u32,
    modifiers: NidaModifiers,
    output: String,
}

static NIDA_KEYMAP: OnceLock<Vec<NidaKeymapEntry>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NidaModifiers {
    Base,
    Shift,
    AltGr,
}

pub(crate) fn lookup_nida_output(keyval: u32, keycode: u32, modifiers: NidaModifiers) -> Option<&'static str> {
    let scancode = scancode_from_keycode(keycode).or_else(|| scancode_from_keyval(keyval))?;
    NIDA_KEYMAP
        .get_or_init(parse_nida_keymap)
        .iter()
        .find(|entry| entry.scancode == scancode && entry.modifiers == modifiers)
        .map(|entry| entry.output.as_str())
}

fn parse_nida_keymap() -> Vec<NidaKeymapEntry> {
    NIDA_KEYMAP_CSV
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut parts = line.splitn(5, ',');
            let _key = parts.next()?.trim();
            let scancode = u32::from_str_radix(parts.next()?.trim(), 16).ok()?;
            let modifiers = parse_modifiers(parts.next()?.trim())?;
            let output = parse_codepoints(parts.next()?.trim())?;
            if output.is_empty() {
                return None;
            }
            Some(NidaKeymapEntry {
                scancode,
                modifiers,
                output,
            })
        })
        .collect()
}

fn parse_modifiers(raw: &str) -> Option<NidaModifiers> {
    match raw {
        "base" => Some(NidaModifiers::Base),
        "shift" => Some(NidaModifiers::Shift),
        "altgr" => Some(NidaModifiers::AltGr),
        _ => None,
    }
}

fn parse_codepoints(raw: &str) -> Option<String> {
    raw.split_whitespace()
        .map(|part| u32::from_str_radix(part, 16).ok().and_then(char::from_u32))
        .collect()
}

/// Map a printable ASCII keyval to its canonical scancode.
///
/// This is the fallback for adapters that only deliver a translated character
/// (no usable scancode). It mirrors the main typing block of a US layout, which
/// is the layout the NIDA keymap was captured against.
fn scancode_from_keyval(keyval: u32) -> Option<u32> {
    let ch = char::from_u32(keyval)?;
    let scancode = match ch.to_ascii_lowercase() {
        '`' | '~' => 41,
        '1' | '!' => 2,
        '2' | '@' => 3,
        '3' | '"' => 4,
        '4' | '$' => 5,
        '5' | '%' => 6,
        '6' | '^' => 7,
        '7' | '&' => 8,
        '8' | '*' => 9,
        '9' | '(' => 10,
        '0' | ')' => 11,
        '-' | '_' => 12,
        '=' | '+' => 13,
        'q' => 16,
        'w' => 17,
        'e' => 18,
        'r' => 19,
        't' => 20,
        'y' => 21,
        'u' => 22,
        'i' => 23,
        'o' => 24,
        'p' => 25,
        '[' | '{' => 26,
        ']' | '}' => 27,
        '\\' | '|' => 43,
        'a' => 30,
        's' => 31,
        'd' => 32,
        'f' => 33,
        'g' => 34,
        'h' => 35,
        'j' => 36,
        'k' => 37,
        'l' => 38,
        ';' | ':' => 39,
        '\'' => 40,
        'z' => 44,
        'x' => 45,
        'c' => 46,
        'v' => 47,
        'b' => 48,
        'n' => 49,
        'm' => 50,
        ',' | '<' => 51,
        '.' | '>' => 52,
        '/' | '?' => 53,
        ' ' => 57,
        _ => return None,
    };
    Some(scancode)
}

/// Accept an adapter-supplied keycode as a canonical scancode.
///
/// Adapters normalize their native key codes into the set-1 / evdev numbering
/// of the main typing block before calling the session. Linux IBus already
/// passes evdev keycodes, which equal these scancodes; the Windows TSF adapter
/// forwards its `lParam` scan code. Only codes the keymap can possibly use are
/// accepted, so non-printable keys (e.g. Backspace=14, Enter=28) fall through to
/// the keyval path rather than colliding with a printable scancode.
fn scancode_from_keycode(keycode: u32) -> Option<u32> {
    match keycode {
        2..=13 | 16..=27 | 30..=40 | 41 | 43..=53 | 57 => Some(keycode),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{lookup_nida_output, NidaModifiers};
    use crate::adapter_contract::InputMode;
    use crate::test_support::session;

    // --- keymap table lookups ---

    #[test]
    fn lookup_uses_lowercase_for_capslock_keyvals() {
        assert_eq!(lookup_nida_output('A' as u32, 0, NidaModifiers::Base), Some("ា"));
    }

    #[test]
    fn lookup_uses_shift_state_for_shifted_letters() {
        assert_eq!(lookup_nida_output('A' as u32, 0, NidaModifiers::Shift), Some("ាំ"));
    }

    #[test]
    fn lookup_prefers_physical_keycode_over_shifted_symbol_keyval() {
        assert_eq!(lookup_nida_output('"' as u32, 3, NidaModifiers::Shift), Some("ៗ"));
    }

    #[test]
    fn lookup_uses_nida_xml_altgr_rows() {
        assert_eq!(
            lookup_nida_output(' ' as u32, 57, NidaModifiers::AltGr),
            Some("\u{00a0}")
        );
    }

    #[test]
    fn lookup_uses_nida_xml_shift_space_row() {
        assert_eq!(lookup_nida_output(' ' as u32, 57, NidaModifiers::Shift), Some(" "));
    }

    #[test]
    fn lookup_uses_canonical_scancodes_for_top_letter_row() {
        let row = [
            ('q', 16, "ឆ"),
            ('w', 17, "ឹ"),
            ('e', 18, "េ"),
            ('r', 19, "រ"),
            ('t', 20, "ត"),
            ('y', 21, "យ"),
            ('u', 22, "ុ"),
            ('i', 23, "ិ"),
            ('o', 24, "ោ"),
            ('p', 25, "ផ"),
        ];
        for (keyval, scancode, expected) in row {
            assert_eq!(
                lookup_nida_output(keyval as u32, scancode, NidaModifiers::Base),
                Some(expected),
                "scancode {scancode} should resolve from the keymap table"
            );
        }
    }

    #[test]
    fn lookup_falls_back_to_keyval_when_no_scancode() {
        // Adapters with only a translated char pass keycode 0; the keyval path
        // must still resolve the same Khmer output as the scancode path.
        assert_eq!(lookup_nida_output('q' as u32, 0, NidaModifiers::Base), Some("ឆ"));
        assert_eq!(lookup_nida_output('k' as u32, 0, NidaModifiers::Base), Some("ក"));
    }

    #[test]
    fn lookup_does_not_map_non_printable_scancodes() {
        // Backspace (14) and Enter (28) sit in the gaps between printable
        // scancode runs and must not collide with a keymap row.
        assert_eq!(lookup_nida_output(0xFF08, 14, NidaModifiers::Base), None);
        assert_eq!(lookup_nida_output(0xFF0D, 28, NidaModifiers::Base), None);
    }

    // --- session behavior in Nida mode ---

    #[test]
    fn nida_mode_commits_mapped_key_without_preedit() {
        let mut session = session();
        session.set_input_mode(InputMode::Nida);

        let update = session.process_key_event('k' as u32, 37, 0);

        assert!(update.consumed);
        assert_eq!(update.commit_text.as_deref(), Some("ក"));
        assert!(!update.history_changed);
        assert!(session.snapshot().raw_preedit.is_empty());
    }

    #[test]
    fn nida_mode_uses_shift_state_for_shifted_output() {
        let mut session = session();
        session.set_input_mode(InputMode::Nida);

        let update = session.process_key_event('K' as u32, 37, 1);

        assert!(update.consumed);
        assert_eq!(update.commit_text.as_deref(), Some("គ"));
    }

    #[test]
    fn nida_mode_ignores_caps_uppercase_for_base_output() {
        let mut session = session();
        session.set_input_mode(InputMode::Nida);

        let update = session.process_key_event('A' as u32, 30, 0);

        assert!(update.consumed);
        assert_eq!(update.commit_text.as_deref(), Some("ា"));
    }

    #[test]
    fn nida_mode_uses_altgr_rows_when_mod5_is_set() {
        let mut session = session();
        session.set_input_mode(InputMode::Nida);

        let update = session.process_key_event(' ' as u32, 57, 1 << 7);

        assert!(update.consumed);
        assert_eq!(update.commit_text.as_deref(), Some("\u{00a0}"));
    }

    #[test]
    fn nida_mode_shift_space_matches_nida_xml() {
        let mut session = session();
        session.set_input_mode(InputMode::Nida);

        let update = session.process_key_event(' ' as u32, 57, 1);

        assert!(update.consumed);
        assert_eq!(update.commit_text.as_deref(), Some(" "));
    }
}
