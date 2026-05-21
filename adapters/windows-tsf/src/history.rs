//! Future Windows history store.
//!
//! Keep the file format compatible with `khmerime_session::HistoryStore` so
//! ranking behavior stays shared across platforms. The planned user-local path
//! is `%APPDATA%\\khmerime\\history.tsv`.
//! TSV is used for runtime history because this is a simple internal key/count
//! store and should not require CSV quoting for Khmer text.

use std::collections::HashMap;

use khmerime_session::should_persist_history_word;

/// Planned Windows user-local history path, expressed with Windows environment syntax.
pub const PLANNED_HISTORY_PATH: &str = "%APPDATA%\\khmerime\\history.tsv";

pub fn parse_history_rows(source: &str) -> HashMap<String, usize> {
    source
        .lines()
        .filter_map(|line| {
            let (word, count) = line.split_once('\t')?;
            let word = word.trim();
            if word.is_empty() || !should_persist_history_word(word) {
                return None;
            }
            let parsed = count.trim().parse::<usize>().ok()?;
            Some((word.to_owned(), parsed))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_history_rows;

    #[test]
    fn parse_history_rows_skips_invalid_and_overlong_words() {
        let parsed = parse_history_rows("ខ្ញុំ\t3\n\t1\nbad\nabcdefghijklmnopqr\t4\nabcdefghijklmnopqrs\t5");

        assert_eq!(parsed.get("ខ្ញុំ"), Some(&3usize));
        assert_eq!(parsed.get("abcdefghijklmnopqr"), Some(&4usize));
        assert_eq!(parsed.get("abcdefghijklmnopqrs"), None);
        assert_eq!(parsed.len(), 2);
    }
}
