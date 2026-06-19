//! CharPick input mode: phonetic Khmer character lookup.
//!
//! Each roman keystroke returns all Khmer characters whose relation list
//! contains that letter. The caller picks one and commits it immediately —
//! no Composition, no preedit accumulation.

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::adapter_contract::SessionResult;
use crate::ime_session::ImeSession;

const CHAR_RELATION_CSV: &str = include_str!("../../../data/khmer_character_relation.csv");
const COENG_SIGN: char = '\u{17D2}';

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
    let mut csv_entries = Vec::new();
    let mut csv_texts = HashSet::new();
    for line in CHAR_RELATION_CSV.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((text, rest)) = line.split_once(',') else {
            continue;
        };
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
            csv_texts.insert(text.to_owned());
            csv_entries.push(CharRelationEntry {
                text: text.to_owned(),
                relations,
            });
        }
    }

    let mut entries = Vec::new();
    for entry in csv_entries {
        let generated_coeng = coeng_form_for_base_consonant(&entry.text).filter(|text| !csv_texts.contains(text));
        let relations = entry.relations.clone();
        entries.push(entry);
        if let Some(text) = generated_coeng {
            entries.push(CharRelationEntry { text, relations });
        }
    }
    entries
}

/// An entry is selectable if it is non-empty. Editorial control lives in the
/// CSV: if a character has a relation entry, the CSV author decided it belongs
/// in the picker.
fn is_selectable(text: &str) -> bool {
    !text.is_empty()
}

fn coeng_form_for_base_consonant(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let ch = chars.next()?;
    if chars.next().is_some() || !is_base_consonant(ch) {
        return None;
    }
    Some(format!("{COENG_SIGN}{ch}"))
}

fn is_base_consonant(ch: char) -> bool {
    ('\u{1780}'..='\u{17A2}').contains(&ch)
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
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
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
    fn k_includes_coeng_ka() {
        let candidates = char_pick_candidates('k');
        assert!(
            candidates.contains(&"្ក"),
            "expected ្ក in candidates for 'k', got: {candidates:?}"
        );
    }

    #[test]
    fn k_includes_subscript_kho() {
        // Subscripts are picked after a base consonant to build clusters
        // (e.g. ត then ្ត for ឧត្តម) — they must be selectable.
        // ្ឃ is the subscript form mapped to [g,k,h] in the CSV.
        let candidates = char_pick_candidates('k');
        assert!(
            candidates.contains(&"្ឃ"),
            "expected ្ឃ in candidates for 'k', got: {candidates:?}"
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
    fn m_includes_nikahit_modifier() {
        // Modifier signs combine with the previous character (e.g. ំ after ុ
        // for the -om sound) — they must be selectable.
        let candidates = char_pick_candidates('m');
        assert!(
            candidates.contains(&"ំ"),
            "expected ំ in candidates for 'm', got: {candidates:?}"
        );
    }

    #[test]
    fn l_includes_punctuation_when_csv_maps_it() {
        // ៘ ("etc." sign) is mapped to [a,l] in the CSV — the CSV author
        // decides what appears; is_selectable no longer blocks any non-empty entry.
        let candidates = char_pick_candidates('l');
        assert!(
            candidates.contains(&"៘"),
            "៘ must appear for 'l' since CSV maps it there, got: {candidates:?}"
        );
    }

    #[test]
    fn a_includes_lek_too_when_csv_maps_it() {
        // ៗ (U+17D7, lek too) is mapped to [a] in the CSV.
        let candidates = char_pick_candidates('a');
        assert!(
            candidates.contains(&"ៗ"),
            "ៗ must appear for 'a' since CSV maps it there, got: {candidates:?}"
        );
    }

    #[test]
    fn unknown_letter_returns_empty() {
        let candidates = char_pick_candidates('z');
        assert!(candidates.is_empty(), "expected empty for 'z', got: {candidates:?}");
    }
}
