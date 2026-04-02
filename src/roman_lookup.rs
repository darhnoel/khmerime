use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "wfst-decoder")]
use crate::decoder::WfstDecoder;
use crate::decoder::{DecoderConfig, DecoderManager, LegacyDecoder, ShadowObservation};

const DEFAULT_DATA: &str = include_str!("../data/roman_lookup_v3.tsv");
const MAX_SUGGESTIONS: usize = 10;
const MAX_MATCHES: usize = 50;
const SYMBOL_DIGITS: [(&str, &str); 10] = [
    ("!", "១"),
    ("\"", "២"),
    ("#", "៣"),
    ("$", "៤"),
    ("%", "៥"),
    ("^", "៦"),
    ("&", "៧"),
    ("*", "៨"),
    ("(", "៩"),
    (")", "០"),
];
const PRIORITY_SEEDS: [(&str, &str); 39] = [
    ("k", "ក"),
    ("kh", "ខ"),
    ("g", "គ"),
    ("gh", "ឃ"),
    ("ng", "ង"),
    ("ch", "ច"),
    ("chh", "ឆ"),
    ("j", "ជ"),
    ("jh", "ឈ"),
    ("nh", "ញ"),
    ("d", "ដ"),
    ("dd", "ឌ"),
    ("ddh", "ឍ"),
    ("n", "ណ"),
    ("t", "ត"),
    ("th", "ថ"),
    ("tt", "ទ"),
    ("tth", "ធ"),
    ("n", "ន"),
    ("b", "ប"),
    ("bh", "ផ"),
    ("p", "ព"),
    ("ph", "ភ"),
    ("m", "ម"),
    ("y", "យ"),
    ("r", "រ"),
    ("l", "ល"),
    ("v", "វ"),
    ("s", "ស"),
    ("h", "ហ"),
    ("l", "ឡ"),
    ("or", "អ"),
    ("a", "អ"),
    ("aa", "អា"),
    ("ae", "ឯ"),
    ("ao", "ឱ"),
    ("av", "អាវ"),
    ("o", "អូន"),
    ("ngg", "ង៉"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub roman: String,
    pub target: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedSuggestion {
    pub text: String,
    pub caret: usize,
}

#[derive(Debug)]
pub enum LexiconError {
    Io(std::io::Error),
    Parse(String),
}

impl fmt::Display for LexiconError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexiconError::Io(error) => write!(f, "{}", error),
            LexiconError::Parse(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for LexiconError {}

impl From<std::io::Error> for LexiconError {
    fn from(error: std::io::Error) -> Self {
        LexiconError::Io(error)
    }
}

pub type Result<T> = std::result::Result<T, LexiconError>;

pub(crate) struct LegacyData {
    entries: Vec<Entry>,
    by_roman: HashMap<String, Vec<String>>,
    index: SearchIndex,
}

pub struct Transliterator {
    legacy: Arc<LegacyData>,
    decoder: DecoderManager,
}

impl Transliterator {
    pub fn from_default_data() -> Result<Self> {
        Self::from_tsv_str_with_config(DEFAULT_DATA, DecoderConfig::default())
    }

    pub fn from_tsv_path(path: impl AsRef<Path>) -> Result<Self> {
        let source = fs::read_to_string(path)?;
        Self::from_tsv_str_with_config(&source, DecoderConfig::default())
    }

    pub fn from_tsv_str(source: &str) -> Result<Self> {
        Self::from_tsv_str_with_config(source, DecoderConfig::default())
    }

    pub fn from_default_data_with_config(config: DecoderConfig) -> Result<Self> {
        Self::from_tsv_str_with_config(DEFAULT_DATA, config)
    }

    pub fn from_tsv_path_with_config(path: impl AsRef<Path>, config: DecoderConfig) -> Result<Self> {
        let source = fs::read_to_string(path)?;
        Self::from_tsv_str_with_config(&source, config)
    }

    pub fn from_tsv_str_with_config(source: &str, config: DecoderConfig) -> Result<Self> {
        let entries = parse_tsv(source)?;
        let legacy = Arc::new(LegacyData::from_entries(entries));
        #[cfg(feature = "wfst-decoder")]
        let decoder = DecoderManager::new(
            LegacyDecoder::new(Arc::clone(&legacy)),
            Some(WfstDecoder::from_entries(legacy.entries())),
            config,
        );
        #[cfg(not(feature = "wfst-decoder"))]
        let decoder = DecoderManager::new(LegacyDecoder::new(Arc::clone(&legacy)), config);
        Ok(Self { legacy, decoder })
    }

    pub fn entries(&self) -> &[Entry] {
        self.legacy.entries()
    }

    pub fn starter_suggestions(&self, history: &HashMap<String, usize>) -> Vec<String> {
        self.legacy.starter_suggestions(history)
    }

    pub fn suggest(&self, input: &str, history: &HashMap<String, usize>) -> Vec<String> {
        debug_assert_eq!(self.decoder.active_decoder_name(), "legacy");
        self.decoder.suggest(input, history)
    }

    pub fn shadow_observation(&self, input: &str, history: &HashMap<String, usize>) -> ShadowObservation {
        self.decoder.shadow_observation(input, history)
    }

    pub fn learn(history: &mut HashMap<String, usize>, suggestion: &str) {
        let count = history.entry(suggestion.to_owned()).or_insert(0);
        *count += 1;
    }

    pub fn token_bounds(text: &str, caret: usize, typed_space: bool) -> Range<usize> {
        let chars = text.chars().collect::<Vec<_>>();
        let end = caret.min(chars.len());
        let mut scan = end.saturating_sub(1);

        if end > 0 && typed_space {
            scan = scan.saturating_sub(1);
        }

        let mut start = scan;
        let mut found_boundary = false;

        while end > 0 && start < chars.len() {
            let ch = chars[start];
            if is_period(ch) {
                found_boundary = true;
                break;
            }
            if !is_roman_letter(ch) {
                start += 1;
                found_boundary = true;
                break;
            }
            if start == 0 {
                found_boundary = true;
                break;
            }
            start -= 1;
        }

        if !found_boundary {
            start = 0;
        }

        start..end
    }

    pub fn apply_suggestion(text: &str, caret: usize, suggestion: &str, typed_space: bool) -> AppliedSuggestion {
        let bounds = Self::token_bounds(text, caret, typed_space);
        let chars = text.chars().collect::<Vec<_>>();
        let replacement_end = (bounds.end + usize::from(typed_space)).min(chars.len());

        let prefix = chars[..bounds.start].iter().collect::<String>();
        let suffix = chars[replacement_end..].iter().collect::<String>();
        let mut output = String::with_capacity(prefix.len() + suggestion.len() + suffix.len());
        output.push_str(&prefix);
        output.push_str(suggestion);
        let caret = output.chars().count();
        output.push_str(&suffix);

        AppliedSuggestion { text: output, caret }
    }
}

impl LegacyData {
    fn from_entries(entries: Vec<Entry>) -> Self {
        let mut by_roman = HashMap::<String, Vec<String>>::new();
        for entry in &entries {
            by_roman
                .entry(entry.roman.clone())
                .or_insert_with(Vec::new)
                .push(entry.target.clone());
        }
        let roman_keys = entries.iter().map(|entry| entry.roman.clone()).collect::<Vec<_>>();
        Self {
            entries,
            by_roman,
            index: SearchIndex::new(&roman_keys, true, 2, 3),
        }
    }

    fn entries(&self) -> &[Entry] {
        &self.entries
    }

    fn starter_suggestions(&self, history: &HashMap<String, usize>) -> Vec<String> {
        let mut suggestions = Vec::new();
        let mut seen = HashSet::new();

        for &(_, target) in &PRIORITY_SEEDS {
            if seen.insert(target) {
                suggestions.push(target.to_owned());
            }
        }

        suggestions.sort_by(|left, right| {
            history
                .get(right)
                .copied()
                .unwrap_or(0)
                .cmp(&history.get(left).copied().unwrap_or(0))
        });
        suggestions.truncate(MAX_SUGGESTIONS);
        suggestions
    }

    pub(crate) fn suggest(&self, input: &str, history: &HashMap<String, usize>) -> Vec<String> {
        let query = input.strip_suffix(' ').unwrap_or(input);
        if query == "." {
            return vec!["។".to_owned(), "៕".to_owned()];
        }
        if let Some((_, digit)) = SYMBOL_DIGITS.iter().find(|(symbol, _)| *symbol == query) {
            return vec![(*digit).to_owned()];
        }
        let normalized = normalize(query);
        if normalized.is_empty() {
            return Vec::new();
        }

        let mut romans = Vec::<String>::new();
        let mut seen_romans = HashSet::<String>::new();

        if let Some(mut matches) = self.index.get(query, 0.33) {
            matches.truncate(MAX_MATCHES);
            for (_, roman) in matches {
                if seen_romans.insert(roman.clone()) {
                    romans.push(roman);
                }
            }
        }

        if normalized.chars().count() <= 1 || romans.is_empty() {
            let mut prefix_matches = self
                .by_roman
                .keys()
                .filter(|roman| normalize(roman).starts_with(&normalized))
                .cloned()
                .collect::<Vec<_>>();
            prefix_matches.sort_by(|left, right| left.len().cmp(&right.len()).then_with(|| left.cmp(right)));

            for roman in prefix_matches {
                if seen_romans.insert(roman.clone()) {
                    romans.push(roman);
                }
                if romans.len() >= MAX_MATCHES {
                    break;
                }
            }
        }

        let mut suggestions = Vec::new();
        let mut seen = HashMap::<String, CandidateMeta>::new();
        let mut visit_index = 0usize;

        if normalized.chars().count() <= 3 {
            for &(roman, target) in &PRIORITY_SEEDS {
                if roman == normalized {
                    push_candidate(
                        &mut suggestions,
                        &mut seen,
                        target,
                        CandidateMeta {
                            exact_match: true,
                            target_len: target.chars().count(),
                            roman_len: roman.chars().count(),
                            visit_index,
                        },
                    );
                    visit_index += 1;
                }
            }
        }

        for roman in romans {
            let roman_normalized = normalize(&roman);
            let exact_match = roman_normalized == normalized;
            let roman_len = roman.chars().count();
            if let Some(values) = self.by_roman.get(&roman) {
                for target in values {
                    push_candidate(
                        &mut suggestions,
                        &mut seen,
                        target,
                        CandidateMeta {
                            exact_match,
                            target_len: target.chars().count(),
                            roman_len,
                            visit_index,
                        },
                    );
                    visit_index += 1;
                    if suggestions.len() >= MAX_SUGGESTIONS {
                        break;
                    }
                }
            }
            if suggestions.len() >= MAX_SUGGESTIONS {
                break;
            }
        }

        suggestions.sort_by(|left, right| {
            history
                .get(right)
                .copied()
                .unwrap_or(0)
                .cmp(&history.get(left).copied().unwrap_or(0))
                .then_with(|| {
                    let left_meta = seen.get(left).copied().unwrap_or_default();
                    let right_meta = seen.get(right).copied().unwrap_or_default();
                    right_meta.cmp_priority(left_meta)
                })
        });
        suggestions.truncate(MAX_SUGGESTIONS);
        suggestions
    }
}

fn push_candidate(
    suggestions: &mut Vec<String>,
    seen: &mut HashMap<String, CandidateMeta>,
    target: &str,
    meta: CandidateMeta,
) {
    match seen.get_mut(target) {
        Some(current) => {
            if meta.better_than(*current) {
                *current = meta;
            }
        }
        None => {
            seen.insert(target.to_owned(), meta);
            suggestions.push(target.to_owned());
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CandidateMeta {
    exact_match: bool,
    target_len: usize,
    roman_len: usize,
    visit_index: usize,
}

impl CandidateMeta {
    fn better_than(self, other: Self) -> bool {
        self.exact_match != other.exact_match && self.exact_match
            || self.target_len < other.target_len
            || (self.target_len == other.target_len && self.roman_len < other.roman_len)
            || (self.target_len == other.target_len
                && self.roman_len == other.roman_len
                && self.visit_index < other.visit_index)
    }

    fn cmp_priority(self, other: Self) -> std::cmp::Ordering {
        self.exact_match
            .cmp(&other.exact_match)
            .then_with(|| other.target_len.cmp(&self.target_len))
            .then_with(|| other.roman_len.cmp(&self.roman_len))
            .then_with(|| other.visit_index.cmp(&self.visit_index))
    }
}

#[derive(Clone, Debug)]
struct SearchIndex {
    gsize_l: usize,
    gsize_u: usize,
    use_levenshtein: bool,
    exact: HashMap<String, String>,
    grams: HashMap<String, Vec<(usize, usize)>>,
    items: BTreeMap<usize, Vec<(f64, String)>>,
}

impl SearchIndex {
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

    fn add(&mut self, item: &str) {
        let normalized = normalize(item);
        if self.exact.contains_key(&normalized) {
            return;
        }
        for size in self.gsize_l..=self.gsize_u {
            self.add_with_size(item, size);
        }
        self.exact.insert(normalized, item.to_owned());
    }

    fn add_with_size(&mut self, item: &str, size: usize) {
        let normalized = normalize(item);
        let grams = ngram_counts(&normalized, size);
        let rows = self.items.entry(size).or_insert_with(Vec::new);
        let row_index = rows.len();
        rows.push((0.0, String::new()));

        let mut magnitude = 0f64;
        for (gram, count) in grams {
            magnitude += (count * count) as f64;
            self.grams.entry(gram).or_insert_with(Vec::new).push((row_index, count));
        }

        rows[row_index] = (magnitude.sqrt(), normalized);
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

fn parse_tsv(source: &str) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for (line_no, line) in source.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let Some((roman, target)) = line.split_once('\t') else {
            return Err(LexiconError::Parse(format!(
                "invalid data format on line {}",
                line_no + 1
            )));
        };
        entries.push(Entry {
            roman: roman.to_owned(),
            target: target.to_owned(),
        });
    }
    Ok(entries)
}

fn is_roman_letter(ch: char) -> bool {
    ch.is_ascii() && ch.is_ascii_alphabetic()
}

fn is_period(ch: char) -> bool {
    ch == '.'
}

pub(crate) fn normalize(input: &str) -> String {
    input
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|ch| {
            ch.is_ascii_alphanumeric()
                || *ch == ','
                || *ch == ' '
                || ('\u{00C0}'..='\u{00FF}').contains(ch)
                || ('\u{0621}'..='\u{064A}').contains(ch)
                || ('\u{0660}'..='\u{0669}').contains(ch)
                || ('\u{1780}'..='\u{17D2}').contains(ch)
        })
        .collect()
}

fn ngram_counts(input: &str, size: usize) -> Vec<(String, usize)> {
    let mut padded = format!("-{}-", normalize(input));
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

    #[test]
    fn loads_embedded_data() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let expected_entries = DEFAULT_DATA.lines().filter(|line| !line.is_empty()).count();
        assert_eq!(transliterator.entries().len(), expected_entries);
        assert!(transliterator
            .entries()
            .iter()
            .any(|entry| entry.roman == "jea" && entry.target == "ជា"));
        assert!(transliterator
            .entries()
            .iter()
            .any(|entry| entry.roman == "tthver" && entry.target == "ធ្វើ"));
    }

    #[test]
    fn reproduces_expected_suggestions() {
        let transliterator = Transliterator::from_default_data().unwrap();
        assert_eq!(
            transliterator.suggest("jea", &HashMap::new()),
            vec!["ជា", "ជះ", "ជាត", "ជាម", "ជាយ", "ជាល", "ជាវ", "ជាតិ", "ជាត់", "ជម្រាប"]
        );
        assert_eq!(
            transliterator.suggest("tver", &HashMap::new()),
            vec!["តើ", "វេរ", "ថេរ", "ទេរ", "ធ្វើ", "ខ្វេរ", "ទ្វារ", "ហ្វឹក", "ថ្នេរ", "ទង្វើ"]
        );
    }

    #[test]
    fn learned_history_reorders_candidates() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let mut history = HashMap::new();
        history.insert("ទេ".to_owned(), 5);
        history.insert("តែ".to_owned(), 1);
        assert_eq!(
            transliterator.suggest("te", &history),
            vec!["ទេ", "តែ", "តើ", "តិះ", "តេត", "តេន", "តិច", "តេជ", "តិចៗ", "ទន្លេ"]
        );
    }

    #[test]
    fn shadow_mode_preserves_legacy_output() {
        let legacy = Transliterator::from_default_data().unwrap();
        let shadow = Transliterator::from_default_data_with_config(
            DecoderConfig::default()
                .with_mode(crate::decoder::DecoderMode::Shadow)
                .with_shadow_log(false),
        )
        .unwrap();

        assert_eq!(
            shadow.suggest("jea", &HashMap::new()),
            legacy.suggest("jea", &HashMap::new())
        );
    }

    #[test]
    fn shadow_observation_is_unavailable_without_wfst_feature() {
        let transliterator = Transliterator::from_default_data_with_config(
            DecoderConfig::default().with_mode(crate::decoder::DecoderMode::Shadow),
        )
        .unwrap();

        let observation = transliterator.shadow_observation("jea", &HashMap::new());
        #[cfg(not(feature = "wfst-decoder"))]
        assert_eq!(observation.mismatch, crate::decoder::ShadowMismatch::WfstUnavailable);
        #[cfg(feature = "wfst-decoder")]
        assert_ne!(observation.mismatch, crate::decoder::ShadowMismatch::WfstUnavailable);
    }

    #[test]
    fn replaces_current_token_like_the_editor() {
        let applied = Transliterator::apply_suggestion("jea ", 4, "ជា", true);
        assert_eq!(
            applied,
            AppliedSuggestion {
                text: "ជា".to_owned(),
                caret: 2,
            }
        );
    }
}
