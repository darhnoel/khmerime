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
const STORAGE_THEME: &str = "roman_lookup.theme";
#[cfg(target_arch = "wasm32")]
const STORAGE_PALETTE: &str = "roman_lookup.palette";
#[cfg(target_arch = "wasm32")]
const STORAGE_SIDEBAR_OPEN: &str = "roman_lookup.sidebar_open";

/// The editor brightness mode. Keep it explicit until following browser/OS
/// changes is reliable across both the app shell and preboot loading screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Theme {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Palette {
    #[default]
    Default,
    Angkor,
    Lotus,
    Forest,
}

impl Palette {
    pub(crate) fn data_attr(self) -> Option<&'static str> {
        match self {
            Palette::Default => None,
            Palette::Angkor => Some("angkor"),
            Palette::Lotus => Some("lotus"),
            Palette::Forest => Some("forest"),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn storage_key(self) -> &'static str {
        self.data_attr().unwrap_or("default")
    }

    #[cfg(target_arch = "wasm32")]
    fn from_storage_key(value: &str) -> Option<Self> {
        match value {
            "default" => Some(Palette::Default),
            "angkor" => Some(Palette::Angkor),
            "lotus" => Some(Palette::Lotus),
            "forest" => Some(Palette::Forest),
            _ => None,
        }
    }
}

impl Theme {
    pub(crate) fn data_attr(self) -> Option<&'static str> {
        match self {
            Theme::Light => Some("light"),
            Theme::Dark => Some("dark"),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn storage_key(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn from_storage_key(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_theme() -> Theme {
    storage_get_web(STORAGE_THEME)
        .as_deref()
        .and_then(Theme::from_storage_key)
        .unwrap_or(Theme::Light)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_theme() -> Theme {
    Theme::Light
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn save_theme(theme: Theme) {
    let _ = storage_set_web(STORAGE_THEME, theme.storage_key());
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_theme(_: Theme) {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_palette() -> Palette {
    storage_get_web(STORAGE_PALETTE)
        .as_deref()
        .and_then(Palette::from_storage_key)
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_palette() -> Palette {
    Palette::default()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn save_palette(palette: Palette) {
    let _ = storage_set_web(STORAGE_PALETTE, palette.storage_key());
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_palette(_: Palette) {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn load_sidebar_open() -> bool {
    if let Some(saved) = storage_get_web(STORAGE_SIDEBAR_OPEN) {
        return saved != "0";
    }
    window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
        .map(|width| width >= 1280.0)
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_sidebar_open() -> bool {
    true
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn save_sidebar_open(open: bool) {
    let _ = storage_set_web(STORAGE_SIDEBAR_OPEN, if open { "1" } else { "0" });
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_sidebar_open(_: bool) {}

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

#[cfg(test)]
mod tests {
    use super::{Palette, Theme};

    #[test]
    fn explicit_themes_map_to_data_attributes() {
        assert_eq!(Theme::Light.data_attr(), Some("light"));
        assert_eq!(Theme::Dark.data_attr(), Some("dark"));
    }

    #[test]
    fn palettes_map_to_independent_root_attributes() {
        assert_eq!(Palette::Default.data_attr(), None);
        assert_eq!(Palette::Angkor.data_attr(), Some("angkor"));
        assert_eq!(Palette::Lotus.data_attr(), Some("lotus"));
        assert_eq!(Palette::Forest.data_attr(), Some("forest"));
    }
}
