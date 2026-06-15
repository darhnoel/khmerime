use serde::{Deserialize, Serialize};

use super::*;

include!(concat!(env!("OUT_DIR"), "/search_index_config.rs"));

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) enum SearchIndex {
    Ngram(NgramSearchIndex),
    SymSpell(SymSpellIndex),
}

impl SearchIndex {
    pub(super) fn new(items: &[String], use_levenshtein: bool, gsize_l: usize, gsize_u: usize) -> Self {
        #[cfg(feature = "no-search-index")]
        {
            let _ = (items, use_levenshtein, gsize_l, gsize_u, DEFAULT_LEGACY_FUZZY_INDEX);
            return Self::Ngram(NgramSearchIndex::empty());
        }
        #[cfg(not(feature = "no-search-index"))]
        {
            if use_symspell_search_index() {
                return Self::SymSpell(SymSpellIndex::new(items, use_levenshtein, 2));
            }
            Self::Ngram(NgramSearchIndex::new(items, use_levenshtein, gsize_l, gsize_u))
        }
    }

    pub(super) fn get(&self, query: &str, threshold: f64) -> Option<Vec<(f64, String)>> {
        match self {
            Self::Ngram(index) => index.get(query, threshold),
            Self::SymSpell(index) => index.get(query, threshold),
        }
    }
}

#[cfg(not(feature = "no-search-index"))]
fn use_symspell_search_index() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return std::env::var("KHMERIME_SEARCH_INDEX")
            .ok()
            .and_then(|value| match value.to_ascii_lowercase().as_str() {
                "symspell" => Some(true),
                "ngram" => Some(false),
                _ => None,
            })
            .unwrap_or_else(|| DEFAULT_LEGACY_FUZZY_INDEX == "symspell");
    }

    #[cfg(target_arch = "wasm32")]
    {
        DEFAULT_LEGACY_FUZZY_INDEX == "symspell"
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct SymSpellIndex {
    use_levenshtein: bool,
    max_edit_distance: usize,
    exact: HashMap<String, usize>,
    deletes: HashMap<String, Vec<usize>>,
    items: Vec<SymSpellItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SymSpellItem {
    normalized: String,
    original: String,
}

impl SymSpellIndex {
    #[cfg(not(feature = "no-search-index"))]
    fn new(items: &[String], use_levenshtein: bool, max_edit_distance: usize) -> Self {
        let mut index = Self {
            use_levenshtein,
            max_edit_distance,
            exact: HashMap::new(),
            deletes: HashMap::new(),
            items: Vec::new(),
        };

        for item in items {
            index.add(item);
        }

        index
    }

    #[cfg(not(feature = "no-search-index"))]
    fn add(&mut self, item: &str) {
        let normalized = normalize(item);
        if self.exact.contains_key(&normalized) {
            return;
        }

        let id = self.items.len();
        for delete in delete_keys(&normalized, self.max_edit_distance) {
            self.deletes.entry(delete).or_insert_with(Vec::new).push(id);
        }
        self.exact.insert(normalized.clone(), id);
        self.items.push(SymSpellItem {
            normalized,
            original: item.to_owned(),
        });
    }

    fn get(&self, query: &str, threshold: f64) -> Option<Vec<(f64, String)>> {
        let normalized = normalize(query);
        if normalized.is_empty() {
            return None;
        }

        let mut candidates = HashSet::<usize>::new();
        if let Some(&id) = self.exact.get(&normalized) {
            candidates.insert(id);
        }
        for delete in delete_keys(&normalized, self.max_edit_distance) {
            if let Some(values) = self.deletes.get(&delete) {
                candidates.extend(values.iter().copied());
            }
        }

        if candidates.is_empty() {
            return None;
        }

        let mut ranked = candidates
            .into_iter()
            .filter_map(|candidate_id| {
                let candidate = &self.items[candidate_id];
                let score = if self.use_levenshtein {
                    similarity(&candidate.normalized, &normalized)
                } else if candidate.normalized == normalized {
                    1.0
                } else {
                    0.0
                };
                if score < threshold {
                    return None;
                }
                Some((score, &candidate.normalized, &candidate.original))
            })
            .collect::<Vec<_>>();

        ranked.sort_by(|left, right| {
            right
                .0
                .total_cmp(&left.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        });

        Some(
            ranked
                .into_iter()
                .map(|(score, _, original)| (score, original.clone()))
                .collect::<Vec<_>>(),
        )
    }
}

fn delete_keys(input: &str, max_distance: usize) -> Vec<String> {
    let chars = input.chars().collect::<Vec<_>>();
    let mut keys = Vec::<String>::with_capacity(delete_key_capacity(chars.len(), max_distance));

    keys.push(chars.iter().collect::<String>());
    if max_distance >= 1 {
        for first in 0..chars.len() {
            keys.push(chars_without(&chars, first, None));
        }
    }
    if max_distance >= 2 {
        for first in 0..chars.len() {
            for second in first + 1..chars.len() {
                keys.push(chars_without(&chars, first, Some(second)));
            }
        }
    }

    keys.sort_unstable();
    keys.dedup();
    keys
}

fn delete_key_capacity(char_count: usize, max_distance: usize) -> usize {
    let mut capacity = 1;
    if max_distance >= 1 {
        capacity += char_count;
    }
    if max_distance >= 2 {
        capacity += char_count.saturating_mul(char_count.saturating_sub(1)) / 2;
    }
    capacity
}

fn chars_without(chars: &[char], first: usize, second: Option<usize>) -> String {
    chars
        .iter()
        .enumerate()
        .filter_map(|(index, ch)| {
            if index == first || second == Some(index) {
                None
            } else {
                Some(*ch)
            }
        })
        .collect::<String>()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct NgramSearchIndex {
    gsize_l: usize,
    gsize_u: usize,
    use_levenshtein: bool,
    exact: HashMap<String, String>,
    grams: HashMap<String, Vec<(usize, usize)>>,
    items: BTreeMap<usize, Vec<(f64, String)>>,
}

impl NgramSearchIndex {
    #[cfg(feature = "no-search-index")]
    fn empty() -> Self {
        Self {
            gsize_l: 2,
            gsize_u: 3,
            use_levenshtein: false,
            exact: HashMap::new(),
            grams: HashMap::new(),
            items: BTreeMap::new(),
        }
    }

    #[cfg(not(feature = "no-search-index"))]
    fn new(items: &[String], use_levenshtein: bool, gsize_l: usize, gsize_u: usize) -> Self {
        let mut index = Self {
            gsize_l,
            gsize_u,
            use_levenshtein,
            exact: HashMap::new(),
            grams: HashMap::new(),
            items: BTreeMap::new(),
        };

        for item in items {
            index.add(item);
        }

        index
    }

    #[cfg(not(feature = "no-search-index"))]
    fn add(&mut self, item: &str) {
        let normalized = normalize(item);
        if self.exact.contains_key(&normalized) {
            return;
        }
        for size in self.gsize_l..=self.gsize_u {
            self.add_with_size(&normalized, size);
        }
        self.exact.insert(normalized, item.to_owned());
    }

    #[cfg(not(feature = "no-search-index"))]
    fn add_with_size(&mut self, normalized: &str, size: usize) {
        let grams = ngram_counts(&normalized, size);
        let rows = self.items.entry(size).or_insert_with(Vec::new);
        let row_index = rows.len();
        rows.push((0.0, String::new()));

        let mut magnitude = 0f64;
        for (gram, count) in grams {
            magnitude += (count * count) as f64;
            self.grams.entry(gram).or_insert_with(Vec::new).push((row_index, count));
        }

        rows[row_index] = (magnitude.sqrt(), normalized.to_owned());
    }

    fn get(&self, query: &str, threshold: f64) -> Option<Vec<(f64, String)>> {
        for size in (self.gsize_l..=self.gsize_u).rev() {
            let matches = self.get_with_size(query, size, threshold);
            if let Some(ref found) = matches {
                if !found.is_empty() {
                    return matches;
                }
            }
        }
        None
    }

    fn get_with_size(&self, query: &str, size: usize, threshold: f64) -> Option<Vec<(f64, String)>> {
        let normalized = normalize(query);
        let grams = ngram_counts(&normalized, size);
        let rows = self.items.get(&size)?;

        let mut scores = HashMap::<usize, usize>::new();
        let mut seen_rows = Vec::<usize>::new();
        let mut magnitude = 0f64;

        for (gram, count) in &grams {
            magnitude += (*count * *count) as f64;
            if let Some(items) = self.grams.get(gram) {
                for &(row, row_count) in items {
                    let entry = scores.entry(row).or_insert_with(|| {
                        seen_rows.push(row);
                        0
                    });
                    *entry += count * row_count;
                }
            }
        }

        if scores.is_empty() {
            return None;
        }

        let query_norm = magnitude.sqrt();
        let mut ranked = seen_rows
            .into_iter()
            .map(|row| {
                let dot = scores[&row];
                let (item_norm, value) = &rows[row];
                (dot as f64 / (query_norm * *item_norm), value.clone())
            })
            .collect::<Vec<_>>();

        ranked.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());

        if self.use_levenshtein {
            ranked = ranked
                .into_iter()
                .take(50)
                .map(|(_, candidate)| (similarity(&candidate, &normalized), candidate))
                .collect::<Vec<_>>();
            ranked.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());
        }

        Some(
            ranked
                .into_iter()
                .filter_map(|(score, candidate)| {
                    if score < threshold {
                        return None;
                    }
                    self.exact.get(&candidate).cloned().map(|original| (score, original))
                })
                .collect(),
        )
    }
}

fn ngram_counts(input: &str, size: usize) -> Vec<(String, usize)> {
    let mut padded = format!("-{input}-");
    if padded.len() < size {
        padded.extend(std::iter::repeat('-').take(size - padded.len()));
    }

    let chars = padded.chars().collect::<Vec<_>>();
    let mut counts = Vec::<(String, usize)>::new();
    let mut positions = HashMap::<String, usize>::new();
    for start in 0..=chars.len().saturating_sub(size) {
        let gram = chars[start..start + size].iter().collect::<String>();
        if let Some(&position) = positions.get(&gram) {
            counts[position].1 += 1;
        } else {
            positions.insert(gram.clone(), counts.len());
            counts.push((gram, 1));
        }
    }
    counts
}

pub(crate) fn similarity(left: &str, right: &str) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }

    let left = left.chars().collect::<Vec<_>>();
    let right = right.chars().collect::<Vec<_>>();
    let mut prev = (0..=left.len()).collect::<Vec<_>>();
    let mut curr = vec![0usize; left.len() + 1];

    for (row, right_char) in right.iter().enumerate() {
        curr[0] = row + 1;
        for (col, left_char) in left.iter().enumerate() {
            curr[col + 1] = if left_char == right_char {
                prev[col]
            } else {
                1 + prev[col].min(prev[col + 1]).min(curr[col])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    let distance = prev[left.len()] as f64;
    let denominator = left.len().max(right.len()) as f64;
    1.0 - distance / denominator
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    fn symspell(items: &[&str]) -> SymSpellIndex {
        SymSpellIndex::new(
            &items.iter().map(|item| (*item).to_owned()).collect::<Vec<_>>(),
            true,
            2,
        )
    }

    fn values(matches: Option<Vec<(f64, String)>>) -> Vec<String> {
        matches
            .unwrap_or_default()
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>()
    }

    #[test]
    fn symspell_exact_query_returns_original_key() {
        let index = symspell(&["Jea"]);

        assert_eq!(values(index.get("jea", 0.33)), vec!["Jea".to_owned()]);
    }

    #[test]
    fn symspell_recovers_one_edit_typo() {
        let index = symspell(&["jea", "tver"]);

        assert_eq!(values(index.get("jra", 0.33)), vec!["jea".to_owned()]);
    }

    #[test]
    fn symspell_recovers_two_edit_typo() {
        let index = symspell(&["abcdef", "uvwxyz"]);

        assert_eq!(values(index.get("abqxef", 0.33)), vec!["abcdef".to_owned()]);
    }

    #[test]
    fn symspell_deduplicates_normalized_items() {
        let index = symspell(&["Jea", "jea"]);

        assert_eq!(values(index.get("jea", 0.33)), vec!["Jea".to_owned()]);
    }

    #[test]
    fn symspell_order_is_deterministic_for_equal_scores() {
        let index = symspell(&["abfde", "abcde"]);

        assert_eq!(
            values(index.get("abxde", 0.33)),
            vec!["abcde".to_owned(), "abfde".to_owned()]
        );
    }

    #[test]
    fn symspell_filters_by_threshold() {
        let index = symspell(&["abcdef"]);

        assert!(values(index.get("abqxef", 0.80)).is_empty());
    }

    #[test]
    #[ignore = "release-only local performance experiment"]
    fn compare_ngram_and_symspell_query_latency() {
        let items = benchmark_roman_keys();
        let queries = benchmark_queries(&items);

        let started = Instant::now();
        let ngram = NgramSearchIndex::new(&items, true, 2, 3);
        let ngram_build = started.elapsed();

        let started = Instant::now();
        let symspell = SymSpellIndex::new(&items, true, 2);
        let symspell_build = started.elapsed();

        let ngram_query = measure_query_latency(&queries, |query| ngram.get(query, 0.33));
        let symspell_query = measure_query_latency(&queries, |query| symspell.get(query, 0.33));

        println!("search-index-bench items={} queries={}", items.len(), queries.len());
        println!("ngram build_ms={:.2}", ngram_build.as_secs_f64() * 1000.0);
        println!("symspell build_ms={:.2}", symspell_build.as_secs_f64() * 1000.0);
        println!(
            "ngram query total_ms={:.2} p50_us={:.2} p95_us={:.2} hits={}",
            ngram_query.total.as_secs_f64() * 1000.0,
            ngram_query.p50.as_secs_f64() * 1_000_000.0,
            ngram_query.p95.as_secs_f64() * 1_000_000.0,
            ngram_query.hits
        );
        println!(
            "symspell query total_ms={:.2} p50_us={:.2} p95_us={:.2} hits={}",
            symspell_query.total.as_secs_f64() * 1000.0,
            symspell_query.p50.as_secs_f64() * 1_000_000.0,
            symspell_query.p95.as_secs_f64() * 1_000_000.0,
            symspell_query.hits
        );
    }

    #[derive(Debug)]
    struct QueryLatency {
        total: Duration,
        p50: Duration,
        p95: Duration,
        hits: usize,
    }

    fn measure_query_latency(
        queries: &[String],
        mut get: impl FnMut(&str) -> Option<Vec<(f64, String)>>,
    ) -> QueryLatency {
        let mut durations = Vec::with_capacity(queries.len());
        let mut hits = 0usize;
        let total_started = Instant::now();
        for query in queries {
            let started = Instant::now();
            let result = get(black_box(query));
            durations.push(started.elapsed());
            hits += result.as_ref().map(Vec::len).unwrap_or(0);
            black_box(result);
        }
        let total = total_started.elapsed();
        durations.sort_unstable();
        QueryLatency {
            total,
            p50: percentile(&durations, 50),
            p95: percentile(&durations, 95),
            hits,
        }
    }

    fn percentile(durations: &[Duration], percentile: usize) -> Duration {
        if durations.is_empty() {
            return Duration::ZERO;
        }
        let index = (durations.len() - 1) * percentile / 100;
        durations[index]
    }

    fn benchmark_roman_keys() -> Vec<String> {
        include_str!("../../../../data/roman_lookup.csv")
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once(',').map(|(roman, _)| normalize(roman)))
            .filter(|roman| {
                let len = roman.chars().count();
                (3..=14).contains(&len)
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    }

    fn benchmark_queries(items: &[String]) -> Vec<String> {
        let mut queries = Vec::new();
        let mut items = items
            .iter()
            .filter(|item| {
                let len = item.chars().count();
                (4..=12).contains(&len)
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.len().cmp(&right.len()).then_with(|| left.cmp(right)));

        for item in items.into_iter().step_by(7).take(1_000) {
            queries.push(item.clone());
            queries.push(delete_middle_char(&item));
            queries.push(replace_middle_char(&item));
            queries.push(replace_two_chars(&item));
        }
        queries
    }

    fn delete_middle_char(input: &str) -> String {
        let chars = input.chars().collect::<Vec<_>>();
        let skip = chars.len() / 2;
        chars
            .into_iter()
            .enumerate()
            .filter_map(|(index, ch)| if index == skip { None } else { Some(ch) })
            .collect::<String>()
    }

    fn replace_middle_char(input: &str) -> String {
        let mut chars = input.chars().collect::<Vec<_>>();
        let index = chars.len() / 2;
        chars[index] = replacement_char(chars[index]);
        chars.into_iter().collect::<String>()
    }

    fn replace_two_chars(input: &str) -> String {
        let mut chars = input.chars().collect::<Vec<_>>();
        let first = chars.len() / 3;
        let second = (chars.len() * 2 / 3).min(chars.len() - 1);
        chars[first] = replacement_char(chars[first]);
        chars[second] = replacement_char(chars[second]);
        chars.into_iter().collect::<String>()
    }

    fn replacement_char(ch: char) -> char {
        match ch {
            'x' => 'z',
            'z' => 'x',
            _ => 'x',
        }
    }
}
