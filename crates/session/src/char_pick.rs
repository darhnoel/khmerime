//! CharPick input mode: phonetic Khmer character lookup.
//!
//! Each roman keystroke returns all Khmer characters whose relation list
//! contains that letter. The caller picks one and commits it immediately —
//! no Composition, no preedit accumulation.

use std::sync::OnceLock;

use crate::adapter_contract::SessionResult;
use crate::ime_session::ImeSession;

const CHAR_RELATION_CSV: &str = include_str!("../../../data/khmer_character_relation.csv");

struct CharRelationEntry {
    text: String,
    /// Roman letters that relate to this character (e.g. ['k', 'h']).
    relations: Vec<char>,
}

static CHAR_RELATIONS: OnceLock<Vec<CharRelationEntry>> = OnceLock::new();

fn relations() -> &'static [CharRelationEntry] {
    CHAR_RELATIONS.get_or_init(parse_relations)
}

fn parse_relations() -> Vec<CharRelationEntry> {
    let mut entries = Vec::new();
    for line in CHAR_RELATION_CSV.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((text, rest)) = line.split_once(',') else { continue };
        if !is_selectable(text) {
            continue;
        }
        // relation column is like `[k,h]` or `[k]`
        let inner = rest.trim().trim_start_matches('[').trim_end_matches(']');
        let relations: Vec<char> = inner
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.chars().next())
            .collect();
        if !relations.is_empty() {
            entries.push(CharRelationEntry { text: text.to_owned(), relations });
        }
    }
    entries
}

/// Returns false for entries that cannot stand alone as a selectable character:
/// - Subscript consonant forms starting with coeng (U+17D2)
/// - Standalone modifier signs (U+17C6–U+17D1): ំ ះ ៈ ៉ ៊ ់ ៌ ៍ ៎ ៏ ័ ៑
/// - Khmer punctuation and currency (U+17D4–U+17DD)
fn is_selectable(text: &str) -> bool {
    match text.chars().next() {
        None => false,
        Some('\u{17D2}') => false,
        Some(c) if ('\u{17C6}'..='\u{17D1}').contains(&c) => false,
        Some(c) if ('\u{17D4}'..='\u{17DD}').contains(&c) => false,
        _ => true,
    }
}

/// Returns all Khmer characters whose relation list contains `letter`.
pub(crate) fn char_pick_candidates(letter: char) -> Vec<&'static str> {
    relations()
        .iter()
        .filter(|e| e.relations.contains(&letter))
        .map(|e| e.text.as_str())
        .collect()
}

impl ImeSession {
    /// Handles a key event while in `InputMode::CharPick`.
    ///
    /// For printable ASCII letters, populates `self.candidates` from the
    /// character-relation lookup and clears any active composition state so
    /// `snapshot()` returns the right candidates with an empty preedit.
    /// Non-printable keys (backspace, enter, etc.) are ignored — the caller
    /// (Swift) handles them directly against the host text field.
    pub(crate) fn process_char_pick_key_event(&mut self, keyval: u32) -> SessionResult {
        let Some(ch) = char::from_u32(keyval).filter(|c| c.is_ascii_alphabetic()) else {
            return SessionResult::default();
        };
        let matches: Vec<String> = char_pick_candidates(ch.to_ascii_lowercase())
            .into_iter()
            .map(|s| s.to_owned())
            .collect();
        // Clear composition so snapshot() shows an empty preedit.
        self.composition_raw.clear();
        self.candidates = matches;
        self.selected_index = 0;
        self.segmented_session = None;
        SessionResult { consumed: true, ..SessionResult::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k_includes_kak() {
        let candidates = char_pick_candidates('k');
        assert!(
            candidates.contains(&"ក"),
            "expected ក in candidates for 'k', got: {candidates:?}"
        );
    }

    #[test]
    fn k_excludes_ma() {
        // ម relates only to [m], never [k]
        let candidates = char_pick_candidates('k');
        assert!(
            !candidates.contains(&"ម"),
            "ម should not appear for 'k', got: {candidates:?}"
        );
    }

    #[test]
    fn h_includes_kho_and_ha() {
        let candidates = char_pick_candidates('h');
        assert!(candidates.contains(&"ខ"), "expected ខ for 'h'");
        assert!(candidates.contains(&"ហ"), "expected ហ for 'h'");
    }

    #[test]
    fn unknown_letter_returns_empty() {
        let candidates = char_pick_candidates('z');
        assert!(candidates.is_empty(), "expected empty for 'z', got: {candidates:?}");
    }
}
