use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use std::{fmt, fs, io, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub next_word: NextWordConfig,
    pub packs: PackConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            next_word: NextWordConfig::default(),
            packs: PackConfig::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextWordConfig {
    pub enabled: bool,
    pub count: usize,
    pub learn_from_typing: bool,
}

impl Default for NextWordConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            count: 5,
            learn_from_typing: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackConfig {
    pub enabled: Vec<String>,
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            enabled: vec!["personal".to_owned()],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconPack {
    pub id: String,
    pub version: String,
    pub entries: HashMap<String, Vec<String>>,
}

pub trait ConfigStore {
    type Error;

    fn load(&self) -> Result<ConfigStoreData, Self::Error>;
    fn save(&self, data: &ConfigStoreData) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigStoreData {
    pub config: Config,
    pub packs: Vec<LexiconPack>,
}

impl Default for ConfigStoreData {
    fn default() -> Self {
        Self {
            config: Config::default(),
            packs: Vec::new(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    TomlDeserialize(toml::de::Error),
    TomlSerialize(toml::ser::Error),
}

#[cfg(not(target_arch = "wasm32"))]
impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::TomlDeserialize(error) => write!(formatter, "{error}"),
            Self::TomlSerialize(error) => write!(formatter, "{error}"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::error::Error for ConfigError {}

#[cfg(not(target_arch = "wasm32"))]
impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        Self::TomlDeserialize(error)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<toml::ser::Error> for ConfigError {
    fn from(error: toml::ser::Error) -> Self {
        Self::TomlSerialize(error)
    }
}

#[cfg(not(target_arch = "wasm32"))]
const CONFIG_FILE: &str = "config.toml";
#[cfg(not(target_arch = "wasm32"))]
const PACKS_DIR: &str = "packs";
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_PACK_VERSION: &str = "1";

#[cfg(not(target_arch = "wasm32"))]
pub fn desktop_config_dir() -> Option<std::path::PathBuf> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(std::path::PathBuf::from(config_home).join("khmerime"));
    }
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| home.join(".config").join("khmerime"))
}

#[cfg(target_arch = "wasm32")]
pub fn desktop_config_dir() -> Option<std::path::PathBuf> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct DesktopConfigStore {
    root: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl DesktopConfigStore {
    pub fn from_dir(path: impl Into<PathBuf>) -> Self {
        Self { root: path.into() }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for DesktopConfigStore {
    fn default() -> Self {
        Self {
            root: desktop_config_dir().unwrap_or_else(|| PathBuf::from(".").join("khmerime")),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ConfigStore for DesktopConfigStore {
    type Error = ConfigError;

    fn load(&self) -> Result<ConfigStoreData, Self::Error> {
        load_config_from_dir(&self.root)
    }

    fn save(&self, data: &ConfigStoreData) -> Result<(), Self::Error> {
        save_config_to_dir(&self.root, data)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_config() -> Result<ConfigStoreData, ConfigError> {
    DesktopConfigStore::default().load()
}

#[cfg(target_arch = "wasm32")]
pub fn load_config() -> Result<ConfigStoreData, std::io::Error> {
    Ok(ConfigStoreData::default())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_config(data: &ConfigStoreData) -> Result<(), ConfigError> {
    DesktopConfigStore::default().save(data)
}

#[cfg(target_arch = "wasm32")]
pub fn save_config(_: &ConfigStoreData) -> Result<(), std::io::Error> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn load_config_from_dir(root: &std::path::Path) -> Result<ConfigStoreData, ConfigError> {
    let config_path = root.join(CONFIG_FILE);
    let config = match fs::read_to_string(config_path) {
        Ok(raw) => toml::from_str::<Config>(&raw)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Config::default(),
        Err(error) => return Err(error.into()),
    };

    let packs = config
        .packs
        .enabled
        .iter()
        .filter_map(|id| load_pack(root, id).transpose())
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ConfigStoreData { config, packs })
}

#[cfg(not(target_arch = "wasm32"))]
fn save_config_to_dir(root: &std::path::Path, data: &ConfigStoreData) -> Result<(), ConfigError> {
    fs::create_dir_all(root.join(PACKS_DIR))?;
    fs::write(root.join(CONFIG_FILE), toml::to_string_pretty(&data.config)?)?;
    for pack in &data.packs {
        fs::write(pack_path(root, &pack.id), serialize_pack_entries(&pack.entries))?;
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn load_pack(root: &std::path::Path, id: &str) -> Result<Option<LexiconPack>, ConfigError> {
    let raw = match fs::read_to_string(pack_path(root, id)) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    Ok(Some(LexiconPack {
        id: id.to_owned(),
        version: DEFAULT_PACK_VERSION.to_owned(),
        entries: parse_pack_entries(&raw),
    }))
}

#[cfg(not(target_arch = "wasm32"))]
fn pack_path(root: &std::path::Path, id: &str) -> PathBuf {
    root.join(PACKS_DIR).join(format!("{id}.tsv"))
}

pub fn normalize_pack_key(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                Some(ch.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect()
}

pub fn parse_pack_entries(source: &str) -> HashMap<String, Vec<String>> {
    let mut entries = HashMap::<String, Vec<String>>::new();
    for line in source.lines() {
        let Some((roman, khmer)) = line.split_once('\t') else {
            continue;
        };
        let roman = normalize_pack_key(roman);
        let khmer = khmer.trim();
        if roman.is_empty() || khmer.is_empty() {
            continue;
        }
        entries.entry(roman).or_default().push(khmer.to_owned());
    }
    for values in entries.values_mut() {
        values.sort();
        values.dedup();
    }
    entries
}

pub fn serialize_pack_entries(entries: &HashMap<String, Vec<String>>) -> String {
    let mut rows = Vec::new();
    let mut keys = entries.keys().collect::<Vec<_>>();
    keys.sort();
    for key in keys {
        let mut values = entries.get(key).cloned().unwrap_or_default();
        values.sort();
        values.dedup();
        for value in values {
            rows.push(format!("{key}\t{value}"));
        }
    }
    rows.join("\n")
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load_config_store_round_trips_enabled_pack_tsvs() {
        let dir = tempfile::tempdir().unwrap();
        let store = DesktopConfigStore::from_dir(dir.path());
        let data = ConfigStoreData {
            config: Config {
                next_word: NextWordConfig {
                    enabled: true,
                    count: 5,
                    learn_from_typing: true,
                },
                packs: PackConfig {
                    enabled: vec!["personal".to_owned(), "tech-terms".to_owned()],
                },
            },
            packs: vec![
                LexiconPack {
                    id: "personal".to_owned(),
                    version: "1".to_owned(),
                    entries: HashMap::from([("knhom".to_owned(), vec!["ខ្ញុំ".to_owned()])]),
                },
                LexiconPack {
                    id: "tech-terms".to_owned(),
                    version: "1".to_owned(),
                    entries: HashMap::from([("cpu".to_owned(), vec!["ស៊ីភីយូ".to_owned()])]),
                },
            ],
        };

        store.save(&data).unwrap();
        assert_eq!(store.load().unwrap(), data);
    }
}
