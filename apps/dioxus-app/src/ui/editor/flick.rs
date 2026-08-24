//! Directional Flick keyboard (ក្តារចុចខ្មែរ) — the direct Khmer input mode.
//!
//! A 4×5 grid of keys; each key is a FAMILY of up to five members reached by a
//! lean from center: center (quick tap), up, left, right, down. Ported from the
//! `khmer-flick-keyboard-cpanel` experiment's frequency-optimized keymap.
//!
//! This module owns the PURE pieces (no DOM): the keymap data and the gesture
//! reducer (pointer origin + current point + a key's members → selected member).
//! The Dioxus component and textarea edit commands live elsewhere.

/// One key: a family of members reached by leaning. An empty string means that
/// direction has no member (a lean toward it stays on center — see `resolve`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Key {
    pub center: &'static str,
    pub up: &'static str,
    pub left: &'static str,
    pub right: &'static str,
    pub down: &'static str,
}

/// The direction a lean resolves to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Center,
    Up,
    Left,
    Right,
    Down,
}

impl Key {
    /// The member glyph for a direction ("" when absent).
    pub(crate) fn member(&self, dir: Direction) -> &'static str {
        match dir {
            Direction::Center => self.center,
            Direction::Up => self.up,
            Direction::Left => self.left,
            Direction::Right => self.right,
            Direction::Down => self.down,
        }
    }

    fn has(&self, dir: Direction) -> bool {
        !self.member(dir).is_empty()
    }
}

const fn key(c: &'static str, u: &'static str, l: &'static str, r: &'static str, d: &'static str) -> Key {
    Key {
        center: c,
        up: u,
        left: l,
        right: r,
        down: d,
    }
}

/// The primary frequency-optimized 4×5 keymap (experiment's `KEYMAP`).
pub(crate) const KEYMAP: [[Key; 5]; 4] = [
    // ROW1 — vowel / ending families
    [
        key("ើ", "", "", "", "ៀ"),
        key("េ", "ៅ", "ៃ", "ែ", "ោ"),
        key("ុ", "", "ឿ", "ួ", "ូ"),
        key("ា", "ឹ", "ី", "ឺ", "ិ"),
        key("ះ", "", "ំ", "", ""),
    ],
    // ROW2 — high-traffic consonant families
    [
        key("ដ", "ណ", "ឍ", "ឌ", "ឋ"),
        key("ក", "ង", "ឃ", "ខ", "គ"),
        key("រ", "ល", "យ", "វ", ""),
        key("ន", "ត", "ធ", "ថ", "ទ"),
        key("ច", "ជ", "ឆ", "ឈ", "ញ"),
    ],
    // ROW3 — coeng bridge + rarer groups
    [
        key("ឯ", "ឧ", "ឱ", "ឬ", "ឥ"),
        key("ស", "អ", "ឡ", "ហ", ""),
        key("្", "់", "៉", "។", ""),
        key("ប", "ម", "ព", "ផ", "ភ"),
        key("ោះ", "េះ", "ុះ", "", ""),
    ],
    // AUXILIARY ROW — signs, composed entries, rare independents
    [
        key("៕", "៚", "៘", "៛", "៙"),
        key("ឲ្យ", "ឦ", "ឩ", "ឳ", "ឨ"),
        key("៏", "័", "៍", "៌", "៊"),
        key("ៈ", "ៗ", "៎", "៖", "៑"),
        key("ឮ", "ឪ", "ឫ", "ឰ", "ឭ"),
    ],
];

/// Neutral-zone radius in px: within this distance of the press origin the key
/// stays on its center member (matches the experiment's 7px threshold).
pub(crate) const NEUTRAL_THRESHOLD_PX: f64 = 7.0;

/// Resolve a lean to the selected member direction.
///
/// `(dx, dy)` is the current point minus the press origin, in px, screen
/// coordinates (y grows downward). Within `NEUTRAL_THRESHOLD_PX` of origin →
/// `Center`. Otherwise the dominant axis picks Up/Down/Left/Right — but a lean
/// toward a direction the key does not have stays on `Center` (never selects an
/// empty member).
pub(crate) fn resolve(key: &Key, dx: f64, dy: f64) -> Direction {
    if dx * dx + dy * dy < NEUTRAL_THRESHOLD_PX * NEUTRAL_THRESHOLD_PX {
        return Direction::Center;
    }
    // Dominant axis: larger absolute component wins.
    let dir = if dx.abs() >= dy.abs() {
        if dx >= 0.0 {
            Direction::Right
        } else {
            Direction::Left
        }
    } else if dy >= 0.0 {
        Direction::Down
    } else {
        Direction::Up
    };
    if key.has(dir) {
        dir
    } else {
        Direction::Center
    }
}

/// The Flick preedit: a stack of atomic Entry Units (each a member glyph,
/// possibly multi-code-point). Backspace pops ONE unit; commit flushes the
/// joined text to the document and clears the stack. Pure — no DOM.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Preedit {
    units: Vec<String>,
}

impl Preedit {
    pub(crate) fn from_text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            units: if text.is_empty() { Vec::new() } else { vec![text] },
        }
    }

    /// Push one selected member as a new Entry Unit. Empty members are ignored
    /// (a center-tap on an empty-center key inserts nothing).
    pub(crate) fn push(&mut self, member: &str) {
        if !member.is_empty() {
            self.units.push(member.to_string());
        }
    }

    /// Remove the last Entry Unit atomically. Returns true if one was removed.
    pub(crate) fn backspace(&mut self) -> bool {
        self.units.pop().is_some()
    }

    /// Remove and return the last Entry Unit so the DOM integration can delete
    /// the same number of code points from the Document atomically.
    pub(crate) fn pop_unit(&mut self) -> Option<String> {
        self.units.pop()
    }

    /// The current preedit text (units joined).
    pub(crate) fn text(&self) -> String {
        self.units.concat()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// Take the preedit text and clear the stack (used on commit/settle).
    pub(crate) fn take(&mut self) -> String {
        let text = self.text();
        self.units.clear();
        text
    }
}

/// The document edit produced by a Flick action: the new full text and where
/// the caret lands (both in char indices). Pure — the caller applies it to the
/// textarea via the platform boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DocEdit {
    pub text: String,
    pub caret: usize,
}

/// Insert `insert` into `text` at char `caret` (replacing the [start, end) char
/// selection when start < end). Caret lands after the inserted text.
pub(crate) fn insert_at(text: &str, sel_start: usize, sel_end: usize, insert: &str) -> DocEdit {
    let chars: Vec<char> = text.chars().collect();
    let start = sel_start.min(chars.len());
    let end = sel_end.min(chars.len()).max(start);
    let mut out: String = chars[..start].iter().collect();
    out.push_str(insert);
    out.extend(chars[end..].iter());
    DocEdit {
        text: out,
        caret: start + insert.chars().count(),
    }
}

/// Delete one character before `caret` (a normal document backspace, used once
/// the Flick preedit is empty). No-op at the start of the document.
pub(crate) fn backspace_at(text: &str, sel_start: usize, sel_end: usize) -> DocEdit {
    let chars: Vec<char> = text.chars().collect();
    let start = sel_start.min(chars.len());
    let end = sel_end.min(chars.len()).max(start);
    if start != end {
        // delete the selection
        let mut out: String = chars[..start].iter().collect();
        out.extend(chars[end..].iter());
        return DocEdit {
            text: out,
            caret: start,
        };
    }
    if start == 0 {
        return DocEdit {
            text: text.to_string(),
            caret: 0,
        };
    }
    let mut out: String = chars[..start - 1].iter().collect();
    out.extend(chars[start..].iter());
    DocEdit {
        text: out,
        caret: start - 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A key with all four leans present, for axis tests.
    const FULL: Key = key("ក", "ង", "ឃ", "ខ", "គ");

    #[test]
    fn within_threshold_stays_center() {
        assert_eq!(resolve(&FULL, 0.0, 0.0), Direction::Center);
        assert_eq!(resolve(&FULL, 5.0, 3.0), Direction::Center); // |v| < 7
    }

    #[test]
    fn dominant_axis_picks_the_direction() {
        assert_eq!(resolve(&FULL, 20.0, 2.0), Direction::Right);
        assert_eq!(resolve(&FULL, -20.0, 2.0), Direction::Left);
        assert_eq!(resolve(&FULL, 2.0, 20.0), Direction::Down);
        assert_eq!(resolve(&FULL, 2.0, -20.0), Direction::Up);
    }

    #[test]
    fn lean_toward_an_empty_direction_stays_center() {
        // រ has no down member.
        let r = key("រ", "ល", "យ", "វ", "");
        assert_eq!(resolve(&r, 2.0, 20.0), Direction::Center);
        // but a real lean still works
        assert_eq!(resolve(&r, -20.0, 2.0), Direction::Left);
    }

    #[test]
    fn member_returns_the_glyph_for_a_direction() {
        assert_eq!(FULL.member(Direction::Center), "ក");
        assert_eq!(FULL.member(Direction::Up), "ង");
        assert_eq!(FULL.member(Direction::Down), "គ");
    }

    #[test]
    fn keymap_is_four_by_five() {
        assert_eq!(KEYMAP.len(), 4);
        assert!(KEYMAP.iter().all(|row| row.len() == 5));
    }

    #[test]
    fn multi_codepoint_members_are_single_entry_units() {
        // ឲ្យ is one Entry Unit though it is several code points.
        assert_eq!(KEYMAP[3][1].center, "ឲ្យ");
        assert!(KEYMAP[3][1].center.chars().count() > 1);
    }

    #[test]
    fn preedit_stacks_units_and_joins_text() {
        let mut p = Preedit::default();
        p.push("ខ");
        p.push("្ញ"); // pretend multi-codepoint unit
        p.push("ុំ");
        assert_eq!(p.text(), "ខ្ញុំ");
    }

    #[test]
    fn backspace_pops_one_entry_unit_atomically() {
        let mut p = Preedit::default();
        p.push("ឲ្យ"); // one multi-code-point unit
        p.push("ក");
        assert!(p.backspace()); // removes ក
        assert_eq!(p.text(), "ឲ្យ");
        assert!(p.backspace()); // removes the whole ឲ្យ unit at once
        assert_eq!(p.text(), "");
        assert!(!p.backspace()); // nothing left
    }

    #[test]
    fn pop_unit_returns_the_complete_last_entry_unit() {
        let mut p = Preedit::default();
        p.push("ក");
        p.push("ឲ្យ");
        assert_eq!(p.pop_unit().as_deref(), Some("ឲ្យ"));
        assert_eq!(p.text(), "ក");
    }

    #[test]
    fn push_ignores_empty_members() {
        let mut p = Preedit::default();
        p.push(""); // center-tap on an empty-center key
        assert!(p.is_empty());
    }

    #[test]
    fn take_returns_text_and_clears() {
        let mut p = Preedit::default();
        p.push("ក");
        p.push("ា");
        assert_eq!(p.take(), "កា");
        assert!(p.is_empty());
    }

    #[test]
    fn insert_at_caret_puts_text_and_advances_caret() {
        // "ខ្ញុំ " is 6 chars; caret at the end (6) inserts after the space.
        let e = insert_at("ខ្ញុំ ", 6, 6, "ទៅ");
        assert_eq!(e.text, "ខ្ញុំ ទៅ");
        assert_eq!(e.caret, 8); // 6 + 2 chars
    }

    #[test]
    fn insert_replaces_a_selection() {
        // select "ab" (chars 0..2) and type ក
        let e = insert_at("abc", 0, 2, "ក");
        assert_eq!(e.text, "កc");
        assert_eq!(e.caret, 1);
    }

    #[test]
    fn insert_multi_codepoint_unit_advances_by_unit_char_len() {
        let e = insert_at("", 0, 0, "ឲ្យ");
        assert_eq!(e.text, "ឲ្យ");
        assert_eq!(e.caret, "ឲ្យ".chars().count());
    }

    #[test]
    fn backspace_deletes_one_char_before_caret() {
        let e = backspace_at("កា", 2, 2);
        assert_eq!(e.text, "ក");
        assert_eq!(e.caret, 1);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let e = backspace_at("ក", 0, 0);
        assert_eq!(e.text, "ក");
        assert_eq!(e.caret, 0);
    }

    #[test]
    fn backspace_deletes_a_selection() {
        let e = backspace_at("abcd", 1, 3);
        assert_eq!(e.text, "ad");
        assert_eq!(e.caret, 1);
    }
}
