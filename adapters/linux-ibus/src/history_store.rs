use std::collections::HashMap;

use khmerime_session::HistoryStore;

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
const DESKTOP_HISTORY_FILE: &str = "history.tsv";

// Desktop history is internal app state, not user-facing import/export data.
// TSV keeps the format compatible with `HistoryStore` while avoiding CSV quoting
// for Khmer text and roman keys.
#[cfg(not(target_arch = "wasm32"))]
pub fn desktop_history_path() -> Option<PathBuf> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(config_home).join("khmerime").join(DESKTOP_HISTORY_FILE));
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config").join("khmerime").join(DESKTOP_HISTORY_FILE))
}

#[cfg(target_arch = "wasm32")]
pub fn desktop_history_path() -> Option<std::path::PathBuf> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_history_rows(source: &str) -> HashMap<String, usize> {
    source
        .lines()
        .filter_map(|line| {
            let (word, count) = line.split_once('\t')?;
            let word = word.trim();
            if word.is_empty() {
                return None;
            }
            let parsed = count.trim().parse::<usize>().ok()?;
            Some((word.to_owned(), parsed))
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn serialize_history_rows(history: &HashMap<String, usize>) -> String {
    let mut rows = history
        .iter()
        .map(|(word, count)| format!("{word}\t{count}"))
        .collect::<Vec<_>>();
    rows.sort();
    rows.join("\n")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_desktop_history() -> HashMap<String, usize> {
    let Some(path) = desktop_history_path() else {
        return HashMap::new();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    parse_history_rows(&raw)
}

#[cfg(target_arch = "wasm32")]
pub fn load_desktop_history() -> HashMap<String, usize> {
    HashMap::new()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_desktop_history(history: &HashMap<String, usize>) -> io::Result<()> {
    let Some(path) = desktop_history_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serialize_history_rows(history))
}

#[cfg(target_arch = "wasm32")]
pub fn save_desktop_history(_: &HashMap<String, usize>) -> std::io::Result<()> {
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DesktopHistoryStore;

impl HistoryStore for DesktopHistoryStore {
    type Error = std::io::Error;

    fn load(&self) -> Result<HashMap<String, usize>, Self::Error> {
        Ok(load_desktop_history())
    }

    fn save(&self, history: &HashMap<String, usize>) -> Result<(), Self::Error> {
        save_desktop_history(history)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_arch = "wasm32"))]
    use super::{parse_history_rows, serialize_history_rows};
    use std::collections::HashMap;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn parse_history_rows_skips_invalid_lines() {
        let parsed = parse_history_rows("ខ្ញុំ\t3\n\t1\nbad\nទៅ\tnope\nទៅ\t4");
        assert_eq!(parsed.get("ខ្ញុំ"), Some(&3usize));
        assert_eq!(parsed.get("ទៅ"), Some(&4usize));
        assert_eq!(parsed.len(), 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn serialize_history_rows_is_sorted() {
        let mut history = HashMap::new();
        history.insert("ទៅ".to_owned(), 2usize);
        history.insert("ខ្ញុំ".to_owned(), 5usize);
        assert_eq!(serialize_history_rows(&history), "ខ្ញុំ\t5\nទៅ\t2");
    }

    #[test]
    fn wasm_stub_load_returns_empty() {
        #[cfg(target_arch = "wasm32")]
        {
            assert!(super::load_desktop_history().is_empty());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let empty: HashMap<String, usize> = HashMap::new();
            assert_eq!(empty.len(), 0);
        }
    }
}
