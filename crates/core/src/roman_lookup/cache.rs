use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::composer::ComposerTable;

use super::types::LegacyData;

const SCHEMA_VERSION: u32 = 1;
const CACHE_DIR_NAME: &str = "khmerime";
const CACHE_FILE_PREFIX: &str = "shared_data";
const CACHE_FILE_SUFFIX: &str = ".bin";
const GC_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60;
const DISABLE_ENV: &str = "KHMERIME_DISABLE_SHARED_DATA_CACHE";
const INDEX_BACKEND_ENV: &str = "KHMERIME_SEARCH_INDEX";

#[derive(Serialize, Deserialize)]
struct CachedSharedData {
    legacy: LegacyData,
    composer: ComposerTable,
}

#[derive(Serialize)]
struct CachedSharedDataRef<'a> {
    legacy: &'a LegacyData,
    composer: &'a ComposerTable,
}

pub(super) fn disabled() -> bool {
    if cfg!(test) {
        return true;
    }
    std::env::var_os(DISABLE_ENV).is_some()
}

pub(super) fn try_load(key: &str) -> Option<(LegacyData, ComposerTable)> {
    let path = cache_file_path(key)?;
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            if error.kind() != io::ErrorKind::NotFound {
                eprintln!(
                    "[ibus-startup] shared_data_cache.read_failed path={} error={error}",
                    path.display()
                );
            }
            return None;
        }
    };
    match bincode::deserialize::<CachedSharedData>(&bytes) {
        Ok(payload) => Some((payload.legacy, payload.composer)),
        Err(error) => {
            eprintln!(
                "[ibus-startup] shared_data_cache.deserialize_failed path={} error={error}",
                path.display()
            );
            None
        }
    }
}

pub(super) fn save(key: &str, legacy: &LegacyData, composer: &ComposerTable) {
    let Some(path) = cache_file_path(key) else { return };
    let Some(dir) = path.parent() else { return };
    if let Err(error) = fs::create_dir_all(dir) {
        eprintln!(
            "[ibus-startup] shared_data_cache.mkdir_failed dir={} error={error}",
            dir.display()
        );
        return;
    }

    let payload = CachedSharedDataRef { legacy, composer };
    let bytes = match bincode::serialize(&payload) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("[ibus-startup] shared_data_cache.serialize_failed error={error}");
            return;
        }
    };

    let tmp = temp_cache_file_path(&path);
    if let Err(error) = fs::write(&tmp, &bytes) {
        eprintln!(
            "[ibus-startup] shared_data_cache.write_failed path={} error={error}",
            tmp.display()
        );
        return;
    }
    if let Err(error) = fs::rename(&tmp, &path) {
        eprintln!(
            "[ibus-startup] shared_data_cache.rename_failed path={} error={error}",
            path.display()
        );
        let _ = fs::remove_file(&tmp);
        return;
    }

    gc_old_entries(dir, &path);
}

pub(super) fn compute_key() -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&SCHEMA_VERSION.to_le_bytes());
    if let Some(exe_stamp) = current_exe_stamp() {
        hasher.update(&exe_stamp.to_le_bytes());
    }
    let backend = std::env::var(INDEX_BACKEND_ENV).unwrap_or_default();
    hasher.update(backend.as_bytes());
    hasher.update(b"|");
    hasher.finalize().to_hex().to_string()
}

fn current_exe_stamp() -> Option<u128> {
    let exe = std::env::current_exe().ok()?;
    let meta = fs::metadata(&exe).ok()?;
    let modified = meta.modified().ok()?;
    modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos())
}

fn cache_dir() -> Option<PathBuf> {
    let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(xdg)
    } else {
        let home = std::env::var_os("HOME")?;
        PathBuf::from(home).join(".cache")
    };
    Some(base.join(CACHE_DIR_NAME))
}

fn cache_file_path(key: &str) -> Option<PathBuf> {
    Some(cache_dir()?.join(format!("{CACHE_FILE_PREFIX}.{key}{CACHE_FILE_SUFFIX}")))
}

fn temp_cache_file_path(path: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(CACHE_FILE_PREFIX);
    path.with_file_name(format!("{file_name}.{}.{}.tmp", std::process::id(), stamp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roman_lookup::Transliterator;
    use std::collections::HashMap;

    #[test]
    fn round_trip_preserves_suggest_output() {
        let original = Transliterator::from_default_shared_data().expect("build shared data");
        let bytes = bincode::serialize(&CachedSharedDataRef {
            legacy: &original.legacy,
            composer: &original.composer,
        })
        .expect("serialize");
        let restored: CachedSharedData = bincode::deserialize(&bytes).expect("deserialize");

        let original_tr =
            Transliterator::from_shared_data_with_config(&original, crate::decoder::DecoderConfig::default());
        let restored_shared = super::super::types::SharedTransliteratorData {
            legacy: std::sync::Arc::new(restored.legacy),
            composer: std::sync::Arc::new(restored.composer),
        };
        let restored_tr =
            Transliterator::from_shared_data_with_config(&restored_shared, crate::decoder::DecoderConfig::default());

        let history = HashMap::new();
        for input in &["jea", "nih", "tver", "domnos", "pjeatthebbat"] {
            let a = original_tr.suggest(input, &history);
            let b = restored_tr.suggest(input, &history);
            assert_eq!(a, b, "suggest output diverged for input {input:?}");
        }
    }
}

fn gc_old_entries(dir: &Path, keep: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        if path == keep {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };
        if !name.starts_with(CACHE_FILE_PREFIX) || !name.ends_with(CACHE_FILE_SUFFIX) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else { continue };
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age.as_secs() > GC_MAX_AGE_SECS {
            let _ = fs::remove_file(&path);
        }
    }
}
