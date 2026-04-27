//! Future Windows history store.
//!
//! Keep the file format compatible with `khmerime_session::HistoryStore` so
//! ranking behavior stays shared across platforms. The planned user-local path
//! is `%APPDATA%\\khmerime\\history.tsv`.
//! TSV is used for runtime history because this is a simple internal key/count
//! store and should not require CSV quoting for Khmer text.

/// Planned Windows user-local history path, expressed with Windows environment syntax.
pub const PLANNED_HISTORY_PATH: &str = "%APPDATA%\\khmerime\\history.tsv";
