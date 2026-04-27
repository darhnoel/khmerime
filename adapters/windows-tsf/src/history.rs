//! Future Windows history store.
//!
//! Keep the file format compatible with `khmerime_session::HistoryStore` so
//! ranking behavior stays shared across platforms. The planned user-local path
//! is `%APPDATA%\\khmerime\\history.tsv`.

/// Planned Windows user-local history path, expressed with Windows environment syntax.
pub const PLANNED_HISTORY_PATH: &str = "%APPDATA%\\khmerime\\history.tsv";
