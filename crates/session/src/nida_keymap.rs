use std::sync::OnceLock;

const NIDA_LINUX_CSV: &str = include_str!("../../../data/keymaps/nida_linux.csv");

#[derive(Debug)]
struct NidaKeymapEntry {
    key: &'static str,
    modifiers: NidaModifiers,
    output: String,
}

static NIDA_KEYMAP: OnceLock<Vec<NidaKeymapEntry>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NidaModifiers {
    Base,
    Shift,
    AltGr,
}

pub fn lookup_nida_output(keyval: u32, keycode: u32, modifiers: NidaModifiers) -> Option<&'static str> {
    let key = key_identity_from_keycode(keycode).or_else(|| key_identity_from_keyval(keyval))?;
    NIDA_KEYMAP
        .get_or_init(parse_nida_keymap)
        .iter()
        .find(|entry| entry.key == key && entry.modifiers == modifiers)
        .map(|entry| entry.output.as_str())
}

fn parse_nida_keymap() -> Vec<NidaKeymapEntry> {
    NIDA_LINUX_CSV
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut parts = line.splitn(5, ',');
            let key = parts.next()?.trim();
            let _scancode = parts.next()?.trim();
            let modifiers = parse_modifiers(parts.next()?.trim())?;
            let output = parse_codepoints(parts.next()?.trim())?;
            if key.is_empty() || output.is_empty() {
                return None;
            }
            Some(NidaKeymapEntry { key, modifiers, output })
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

fn key_identity_from_keyval(keyval: u32) -> Option<&'static str> {
    let ch = char::from_u32(keyval)?;
    match ch.to_ascii_lowercase() {
        '`' | '~' => Some("grave"),
        '1' | '!' => Some("1"),
        '2' | '@' => Some("2"),
        '3' | '"' => Some("3"),
        '4' | '$' => Some("4"),
        '5' | '%' => Some("5"),
        '6' | '^' => Some("6"),
        '7' | '&' => Some("7"),
        '8' | '*' => Some("8"),
        '9' | '(' => Some("9"),
        '0' | ')' => Some("0"),
        '-' | '_' => Some("minus"),
        '=' | '+' => Some("equal"),
        'q' => Some("q"),
        'w' => Some("w"),
        'e' => Some("e"),
        'r' => Some("r"),
        't' => Some("t"),
        'y' => Some("y"),
        'u' => Some("u"),
        'i' => Some("i"),
        'o' => Some("o"),
        'p' => Some("p"),
        '[' | '{' => Some("bracket_left"),
        ']' | '}' => Some("bracket_right"),
        '\\' | '|' => Some("backslash"),
        'a' => Some("a"),
        's' => Some("s"),
        'd' => Some("d"),
        'f' => Some("f"),
        'g' => Some("g"),
        'h' => Some("h"),
        'j' => Some("j"),
        'k' => Some("k"),
        'l' => Some("l"),
        ';' | ':' => Some("semicolon"),
        '\'' => Some("apostrophe"),
        'z' => Some("z"),
        'x' => Some("x"),
        'c' => Some("c"),
        'v' => Some("v"),
        'b' => Some("b"),
        'n' => Some("n"),
        'm' => Some("m"),
        ',' | '<' => Some("comma"),
        '.' | '>' => Some("period"),
        '/' | '?' => Some("slash"),
        ' ' => Some("space"),
        _ => None,
    }
}

fn key_identity_from_keycode(keycode: u32) -> Option<&'static str> {
    match keycode {
        // IBus forwards Linux evdev keycodes here. Do not use XKB keycodes:
        // that would shift the row by 8, causing Backspace(14) to map to "5",
        // Enter(28) to map to "t", and Q(16) to map to "7".
        41 => Some("grave"),
        2 => Some("1"),
        3 => Some("2"),
        4 => Some("3"),
        5 => Some("4"),
        6 => Some("5"),
        7 => Some("6"),
        8 => Some("7"),
        9 => Some("8"),
        10 => Some("9"),
        11 => Some("0"),
        12 => Some("minus"),
        13 => Some("equal"),
        16 => Some("q"),
        17 => Some("w"),
        18 => Some("e"),
        19 => Some("r"),
        20 => Some("t"),
        21 => Some("y"),
        22 => Some("u"),
        23 => Some("i"),
        24 => Some("o"),
        25 => Some("p"),
        26 => Some("bracket_left"),
        27 => Some("bracket_right"),
        43 => Some("backslash"),
        30 => Some("a"),
        31 => Some("s"),
        32 => Some("d"),
        33 => Some("f"),
        34 => Some("g"),
        35 => Some("h"),
        36 => Some("j"),
        37 => Some("k"),
        38 => Some("l"),
        39 => Some("semicolon"),
        40 => Some("apostrophe"),
        44 => Some("z"),
        45 => Some("x"),
        46 => Some("c"),
        47 => Some("v"),
        48 => Some("b"),
        49 => Some("n"),
        50 => Some("m"),
        51 => Some("comma"),
        52 => Some("period"),
        53 => Some("slash"),
        57 => Some("space"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{lookup_nida_output, NidaModifiers};

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
    fn lookup_uses_ibus_evdev_keycodes_for_top_letter_row() {
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
        for (keyval, keycode, expected) in row {
            assert_eq!(
                lookup_nida_output(keyval as u32, keycode, NidaModifiers::Base),
                Some(expected),
                "keycode {keycode} should map from evdev, not XKB"
            );
        }
    }

    #[test]
    fn lookup_does_not_map_backspace_or_enter_evdev_keycodes() {
        assert_eq!(lookup_nida_output(0xFF08, 14, NidaModifiers::Base), None);
        assert_eq!(lookup_nida_output(0xFF0D, 28, NidaModifiers::Base), None);
    }
}
