use dioxus::prelude::*;

use crate::ui::storage::save_user_dictionary;

use super::candidate_pipeline::{normalize_user_dictionary_key, update_candidates};
use super::{EditorSignals, ManualSaveRequest};

/// Persist one explicit Roman → Khmer mapping from the add-pair modal.
///
/// The modal owns all temporary input. Editor state only receives the finished
/// pair, which keeps Flick input isolated from the document pipeline.
pub(crate) fn save_manual_save_request(request: ManualSaveRequest, state: EditorSignals) -> bool {
    replace_manual_save_request(None, request, state)
}

pub(crate) fn replace_manual_save_request(
    original: Option<ManualSaveRequest>,
    request: ManualSaveRequest,
    mut state: EditorSignals,
) -> bool {
    let key = normalize_user_dictionary_key(&request.roman);
    let khmer = request.khmer.trim();
    if key.is_empty() || khmer.is_empty() {
        return false;
    }

    let mut dictionary = state.user_dictionary();
    if let Some(original) = original {
        let original_key = normalize_user_dictionary_key(&original.roman);
        if let Some(values) = dictionary.get_mut(&original_key) {
            values.retain(|value| value != original.khmer.trim());
            if values.is_empty() {
                dictionary.remove(&original_key);
            }
        }
    }
    let values = dictionary.entry(key).or_default();
    if !values.iter().any(|value| value == khmer) {
        values.insert(0, khmer.to_owned());
    }
    save_user_dictionary(&dictionary);
    state.user_dictionary.set(dictionary);
    if state.roman_enabled() {
        spawn(update_candidates(state.text(), state));
    }
    true
}

pub(crate) fn remove_user_dictionary_mapping(roman: &str, khmer: &str, mut state: EditorSignals) -> bool {
    let key = normalize_user_dictionary_key(roman);
    if key.is_empty() || khmer.trim().is_empty() {
        return false;
    }

    let mut dictionary = state.user_dictionary();
    let mut changed = false;
    if let Some(values) = dictionary.get_mut(&key) {
        let before = values.len();
        values.retain(|value| value != khmer);
        changed = values.len() != before;
        if values.is_empty() {
            dictionary.remove(&key);
        }
    }
    if !changed {
        return false;
    }

    save_user_dictionary(&dictionary);
    state.user_dictionary.set(dictionary);

    if state.roman_enabled() {
        spawn(update_candidates(state.text(), state));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::normalize_user_dictionary_key;

    #[test]
    fn normalizes_saved_roman_keys() {
        assert_eq!(normalize_user_dictionary_key("  KhNhom  "), "khnhom");
    }
}
