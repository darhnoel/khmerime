use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use khmerime_linux_ibus::{load_desktop_history, save_desktop_history};
use roman_lookup::DecoderMode;

#[cfg(target_arch = "wasm32")]
use web_sys::window;

#[cfg(target_arch = "wasm32")]
const STORAGE_TEXT: &str = "roman_lookup.text";
#[cfg(target_arch = "wasm32")]
const STORAGE_ENABLED: &str = "roman_lookup.enabled";
#[cfg(target_arch = "wasm32")]
const STORAGE_HISTORY: &str = "roman_lookup.history";
#[cfg(target_arch = "wasm32")]
const STORAGE_USER_DICTIONARY: &str = "roman_lookup.user_dictionary";
#[cfg(target_arch = "wasm32")]
const STORAGE_FONT_SIZE: &str = "roman_lookup.font_size";

#[cfg(target_arch = "wasm32")]
fn storage_get_web(key: &str) -> Option<String> {
    let storage = window()?.local_storage().ok().flatten()?;
    storage.get_item(key).ok().flatten()
}

#[cfg(target_arch = "wasm32")]
fn storage_set_web(key: &str, value: &str) -> Option<()> {
    let storage = window()?.local_storage().ok().flatten()?;
    storage.set_item(key, value).ok()?;
    Some(())
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_editor_text() -> String {
    storage_get_web(STORAGE_TEXT).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_editor_text() -> String {
    String::new()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn save_editor_text(value: &str) {
    let _ = storage_set_web(STORAGE_TEXT, value);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_editor_text(_: &str) {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_enabled() -> bool {
    storage_get_web(STORAGE_ENABLED)
        .map(|value| value != "0")
        .unwrap_or(true)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_enabled() -> bool {
    true
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn save_enabled(value: bool) {
    let _ = storage_set_web(STORAGE_ENABLED, if value { "1" } else { "0" });
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_enabled(_: bool) {}

pub(crate) fn load_decoder_mode() -> DecoderMode {
    // Keep shadow as the configured mode; startup still uses legacy behavior until
    // full engine readiness gates are satisfied in candidate_pipeline.
    DecoderMode::Shadow
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_history() -> HashMap<String, usize> {
    storage_get_web(STORAGE_HISTORY)
        .map(|raw| {
            raw.lines()
                .filter_map(|line| {
                    let (word, count) = line.split_once('\t')?;
                    Some((word.to_owned(), count.parse().ok()?))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_history() -> HashMap<String, usize> {
    load_desktop_history()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn save_history(history: &HashMap<String, usize>) {
    let mut rows = history
        .iter()
        .map(|(word, count)| format!("{word}\t{count}"))
        .collect::<Vec<_>>();
    rows.sort();
    let _ = storage_set_web(STORAGE_HISTORY, &rows.join("\n"));
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_history(history: &HashMap<String, usize>) {
    let _ = save_desktop_history(history);
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_user_dictionary() -> HashMap<String, Vec<String>> {
    storage_get_web(STORAGE_USER_DICTIONARY)
        .map(|raw| {
            let mut dictionary = HashMap::<String, Vec<String>>::new();
            for line in raw.lines() {
                let Some((roman, khmer)) = line.split_once('\t') else {
                    continue;
                };
                let roman = roman.trim();
                let khmer = khmer.trim();
                if roman.is_empty() || khmer.is_empty() {
                    continue;
                }
                dictionary.entry(roman.to_owned()).or_default().push(khmer.to_owned());
            }
            for values in dictionary.values_mut() {
                values.sort();
                values.dedup();
            }
            dictionary
        })
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_user_dictionary() -> HashMap<String, Vec<String>> {
    HashMap::new()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn save_user_dictionary(dictionary: &HashMap<String, Vec<String>>) {
    let mut rows = Vec::new();
    let mut keys = dictionary.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    for key in keys {
        let mut values = dictionary.get(&key).cloned().unwrap_or_default();
        values.sort();
        values.dedup();
        for value in values {
            rows.push(format!("{key}\t{value}"));
        }
    }
    let _ = storage_set_web(STORAGE_USER_DICTIONARY, &rows.join("\n"));
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_user_dictionary(_: &HashMap<String, Vec<String>>) {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_font_size(min_font_size: usize, max_font_size: usize, default_font_size: usize) -> usize {
    storage_get_web(STORAGE_FONT_SIZE)
        .and_then(|value| value.parse::<usize>().ok())
        .map(|value| value.clamp(min_font_size, max_font_size))
        .unwrap_or(default_font_size)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_font_size(_: usize, _: usize, default_font_size: usize) -> usize {
    default_font_size
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn save_font_size(value: usize, min_font_size: usize, max_font_size: usize) {
    let _ = storage_set_web(
        STORAGE_FONT_SIZE,
        &value.clamp(min_font_size, max_font_size).to_string(),
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_font_size(_: usize, _: usize, _: usize) {}
