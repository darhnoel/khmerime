use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use crate::composer::ComposerTable;
use crate::decoder::{DecoderConfig, DecoderManager, DecoderMode, LegacyDecoder, ShadowObservation, WfstDecoder};

const DEFAULT_COMPILED_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/roman_lookup.lexicon.bin"));
const DEFAULT_COMPILED_KHPOS_STATS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/khpos.stats.bin"));
const COMPILED_MAGIC: &[u8; 4] = b"RLX1";
const KHPOS_MAGIC: &[u8; 4] = b"KPS1";
const MAX_SUGGESTIONS: usize = 15;
const MAX_MATCHES: usize = 12;
const KEYCAP_SUGGESTIONS: [(&str, &str); 21] = [
    ("1", "១"),
    ("!", "!"),
    ("2", "២"),
    ("\"", "ៗ"),
    ("3", "៣"),
    ("#", "\""),
    ("4", "៤"),
    ("$", "៛"),
    ("5", "៥"),
    ("%", "%"),
    ("6", "៦"),
    ("&", "៍"),
    ("7", "៧"),
    ("'", "័"),
    ("8", "៨"),
    ("(", "៏"),
    ("9", "៩"),
    (")", "("),
    ("0", "០"),
    ("~", ")"),
    ("=", "៌"),
];

fn khmer_digit(ch: char) -> Option<char> {
    match ch {
        '0' => Some('០'),
        '1' => Some('១'),
        '2' => Some('២'),
        '3' => Some('៣'),
        '4' => Some('៤'),
        '5' => Some('៥'),
        '6' => Some('៦'),
        '7' => Some('៧'),
        '8' => Some('៨'),
        '9' => Some('៩'),
        _ => None,
    }
}
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

#[derive(Clone, Debug)]
pub(crate) struct RankedLexiconEntry {
    pub target: String,
    pub canonical_roman: String,
    pub normalized_key: String,
    pub alias_keys: Vec<String>,
    pub frequency: u32,
    pub source_rank: usize,
    pub first_tag: Option<String>,
    pub last_tag: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CorpusStats {
    #[allow(dead_code)]
    pub word_unigrams: HashMap<String, u32>,
    #[allow(dead_code)]
    pub word_bigrams: HashMap<(String, String), u32>,
    pub surface_unigrams: HashMap<String, u32>,
    pub tag_unigrams: HashMap<String, u32>,
    pub tag_bigrams: HashMap<(String, String), u32>,
    pub dominant_word_tags: HashMap<String, DominantTag>,
}

#[derive(Clone, Debug)]
pub(crate) struct DominantTag {
    pub tag: String,
    #[allow(dead_code)]
    pub support: u32,
}

impl CorpusStats {
    fn from_default_data() -> Result<Self> {
        parse_compiled_khpos_stats(DEFAULT_COMPILED_KHPOS_STATS)
    }

    fn dominant_tag(&self, word: &str) -> Option<&str> {
        self.dominant_word_tags.get(word).map(|entry| entry.tag.as_str())
    }
}

impl RankedLexiconEntry {
    pub(crate) fn score_forms(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.normalized_key.as_str())
            .chain(self.alias_keys.iter().map(String::as_str))
            .filter(|key| !key.starts_with("sk:"))
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RankedLexicon {
    pub entries: Vec<RankedLexiconEntry>,
    pub exact_index: HashMap<String, Vec<usize>>,
    pub alias_index: HashMap<String, Vec<usize>>,
    pub gram_index: HashMap<String, Vec<usize>>,
    pub word_unigrams: HashMap<String, u32>,
    pub word_bigrams: HashMap<(String, String), u32>,
    pub corpus_word_unigrams: HashMap<String, u32>,
    pub corpus_word_bigrams: HashMap<(String, String), u32>,
    pub corpus_surface_unigrams: HashMap<String, u32>,
    pub tag_unigrams: HashMap<String, u32>,
    pub tag_bigrams: HashMap<(String, String), u32>,
}

pub(crate) struct LegacyData {
    entries: Vec<Entry>,
    by_roman: HashMap<String, Vec<String>>,
    by_normalized: HashMap<String, Vec<String>>,
    by_target: HashMap<String, Vec<String>>,
    roman_normalized: HashMap<String, String>,
    roman_prefix_index: HashMap<String, Vec<String>>,
    index: SearchIndex,
    ranked: RankedLexicon,
}

pub struct Transliterator {
    legacy: Arc<LegacyData>,
    decoder: DecoderManager,
}

impl Transliterator {
    pub fn from_default_data() -> Result<Self> {
        Self::from_default_data_with_config(DecoderConfig::default())
    }

    pub fn from_tsv_path(path: impl AsRef<Path>) -> Result<Self> {
        let source = fs::read_to_string(path)?;
        Self::from_tsv_str_with_config(&source, DecoderConfig::default())
    }

    pub fn from_tsv_str(source: &str) -> Result<Self> {
        Self::from_tsv_str_with_config(source, DecoderConfig::default())
    }

    pub fn from_default_data_with_config(config: DecoderConfig) -> Result<Self> {
        let entries = parse_compiled_lexicon(DEFAULT_COMPILED_DATA)?;
        Self::from_entries_with_config(entries, config)
    }

    pub fn from_tsv_path_with_config(path: impl AsRef<Path>, config: DecoderConfig) -> Result<Self> {
        let source = fs::read_to_string(path)?;
        Self::from_tsv_str_with_config(&source, config)
    }

    pub fn from_tsv_str_with_config(source: &str, config: DecoderConfig) -> Result<Self> {
        let entries = parse_tsv(source)?;
        Self::from_entries_with_config(entries, config)
    }

    fn from_entries_with_config(entries: Vec<Entry>, config: DecoderConfig) -> Result<Self> {
        let legacy = Arc::new(LegacyData::from_entries(entries));
        let composer = ComposerTable::from_entries(legacy.entries());
        let decoder = DecoderManager::new(
            composer,
            LegacyDecoder::new(Arc::clone(&legacy)),
            (config.mode != DecoderMode::Legacy).then(|| WfstDecoder::new(Arc::clone(&legacy), config.clone())),
            config,
        );
        Ok(Self { legacy, decoder })
    }

    pub fn entries(&self) -> &[Entry] {
        self.legacy.entries()
    }

    pub fn starter_suggestions(&self, history: &HashMap<String, usize>) -> Vec<String> {
        self.legacy.starter_suggestions(history)
    }

    pub fn suggest(&self, input: &str, history: &HashMap<String, usize>) -> Vec<String> {
        self.decoder.suggest(input, history)
    }

    pub fn exact_match_targets(&self, input: &str) -> Vec<String> {
        let query = input.strip_suffix(' ').unwrap_or(input);
        let normalized = normalize(query);
        if normalized.is_empty() {
            return Vec::new();
        }
        self.legacy
            .exact_targets(&normalized)
            .map_or_else(Vec::new, |targets| targets.to_vec())
    }

    pub fn shadow_observation(&self, input: &str, history: &HashMap<String, usize>) -> ShadowObservation {
        self.decoder.shadow_observation(input, history)
    }

    pub fn best_prefix_consumption(&self, input: &str, target: &str) -> Option<String> {
        self.legacy.best_prefix_consumption(input, target)
    }

    pub fn learn(history: &mut HashMap<String, usize>, suggestion: &str) {
        let count = history.entry(suggestion.to_owned()).or_insert(0);
        *count += 1;
    }

    pub fn token_bounds(text: &str, caret: usize, typed_space: bool) -> Range<usize> {
        let chars = text.chars().collect::<Vec<_>>();
        let end = caret.min(chars.len());
        if end == 0 {
            return 0..0;
        }
        let mut scan = end.saturating_sub(1);

        if end > 0 && typed_space {
            scan = scan.saturating_sub(1);
        }
        if scan >= chars.len() {
            return end..end;
        }
        if is_period(chars[scan]) {
            return scan..end;
        }

        let is_token_char = if is_roman_letter(chars[scan]) {
            is_roman_letter as fn(char) -> bool
        } else if is_keycap_token_char(chars[scan]) {
            is_keycap_token_char as fn(char) -> bool
        } else {
            return end..end;
        };

        let mut start = scan;
        while start > 0 {
            let previous = chars[start - 1];
            if !is_token_char(previous) {
                break;
            }
            start -= 1;
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
    pub(crate) fn from_entries(entries: Vec<Entry>) -> Self {
        let corpus_stats = CorpusStats::from_default_data().expect("embedded khPOS stats must load");
        let mut by_roman = HashMap::<String, Vec<String>>::new();
        let mut by_normalized = HashMap::<String, Vec<String>>::new();
        let mut by_target = HashMap::<String, Vec<String>>::new();
        for entry in &entries {
            by_roman
                .entry(entry.roman.clone())
                .or_insert_with(Vec::new)
                .push(entry.target.clone());
            by_normalized
                .entry(normalize(&entry.roman))
                .or_insert_with(Vec::new)
                .push(entry.target.clone());
            by_target
                .entry(entry.target.clone())
                .or_insert_with(Vec::new)
                .push(normalize(&entry.roman));
        }
        let mut sorted_romans = by_roman.keys().cloned().collect::<Vec<_>>();
        sorted_romans.sort_by(|left, right| left.len().cmp(&right.len()).then_with(|| left.cmp(right)));
        let mut roman_normalized = HashMap::<String, String>::new();
        let mut roman_prefix_index = HashMap::<String, Vec<String>>::new();
        for roman in &sorted_romans {
            let normalized = normalize(roman);
            roman_normalized.insert(roman.clone(), normalized.clone());
            for prefix_len in 1..=3 {
                let prefix = normalized.chars().take(prefix_len).collect::<String>();
                if prefix.chars().count() != prefix_len {
                    break;
                }
                roman_prefix_index
                    .entry(prefix)
                    .or_insert_with(Vec::new)
                    .push(roman.clone());
            }
        }
        let roman_keys = entries.iter().map(|entry| entry.roman.clone()).collect::<Vec<_>>();
        let ranked = RankedLexicon::from_entries(&entries, &corpus_stats);
        Self {
            entries,
            by_roman,
            by_normalized,
            by_target,
            roman_normalized,
            roman_prefix_index,
            index: SearchIndex::new(&roman_keys, true, 2, 3),
            ranked,
        }
    }

    fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub(crate) fn ranked(&self) -> &RankedLexicon {
        &self.ranked
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

    fn best_prefix_consumption(&self, input: &str, target: &str) -> Option<String> {
        let normalized_input = normalize(input);
        if normalized_input.is_empty() {
            return None;
        }

        let mut matches = self
            .by_target
            .get(target)?
            .iter()
            .filter(|roman| !roman.is_empty() && normalized_input.starts_with(roman.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
        matches.dedup();
        matches.into_iter().next()
    }

    pub(crate) fn suggest(&self, input: &str, history: &HashMap<String, usize>) -> Vec<String> {
        let query = input.strip_suffix(' ').unwrap_or(input);
        if query == "." {
            return vec!["។".to_owned(), "៕".to_owned()];
        }
        if query.chars().all(|ch| ch.is_ascii_digit()) && !query.is_empty() {
            let mapped = query.chars().filter_map(khmer_digit).collect::<String>();
            if !mapped.is_empty() {
                return vec![mapped];
            }
        }
        if let Some((_, mapped)) = KEYCAP_SUGGESTIONS.iter().find(|(key, _)| *key == query) {
            return vec![(*mapped).to_owned()];
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
            let prefix_seed = normalized.chars().take(3).collect::<String>();
            let seed_pool = self
                .roman_prefix_index
                .get(&prefix_seed)
                .cloned()
                .unwrap_or_else(|| self.by_roman.keys().cloned().collect::<Vec<_>>());
            let prefix_matches = seed_pool
                .into_iter()
                .filter(|roman| {
                    self.roman_normalized
                        .get(roman)
                        .map(|value| value.starts_with(&normalized))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();

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
            let exact_match = self
                .roman_normalized
                .get(&roman)
                .map(|value| value == &normalized)
                .unwrap_or(false);
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

    pub(crate) fn exact_targets(&self, normalized: &str) -> Option<&[String]> {
        self.by_normalized.get(normalized).map(Vec::as_slice)
    }
}

impl RankedLexicon {
    fn from_entries(entries: &[Entry], corpus_stats: &CorpusStats) -> Self {
        let mut ranked = Self::default();
        let mut target_frequency = HashMap::<String, u32>::new();
        for entry in entries {
            *target_frequency.entry(entry.target.clone()).or_default() += 1;
            let words = entry.target.split_whitespace().collect::<Vec<_>>();
            for word in &words {
                *ranked.word_unigrams.entry((*word).to_owned()).or_default() += 1;
            }
            for pair in words.windows(2) {
                *ranked
                    .word_bigrams
                    .entry((pair[0].to_owned(), pair[1].to_owned()))
                    .or_default() += 1;
            }
        }

        ranked.corpus_word_unigrams = corpus_stats.word_unigrams.clone();
        ranked.corpus_word_bigrams = corpus_stats.word_bigrams.clone();
        ranked.corpus_surface_unigrams = corpus_stats.surface_unigrams.clone();
        ranked.tag_unigrams = corpus_stats.tag_unigrams.clone();
        ranked.tag_bigrams = corpus_stats.tag_bigrams.clone();

        for (source_rank, entry) in entries.iter().enumerate() {
            let normalized_key = normalize(&entry.roman);
            if normalized_key.is_empty() {
                continue;
            }
            let alias_keys = roman_search_variants(&entry.roman)
                .into_iter()
                .filter(|key| key != &normalized_key)
                .collect::<Vec<_>>();
            let (first_tag, last_tag) = boundary_tags_for_target(&entry.target, corpus_stats);
            let ranked_entry = RankedLexiconEntry {
                target: entry.target.clone(),
                canonical_roman: entry.roman.clone(),
                normalized_key: normalized_key.clone(),
                alias_keys: alias_keys.clone(),
                frequency: target_frequency.get(&entry.target).copied().unwrap_or(1),
                source_rank,
                first_tag,
                last_tag,
            };
            let entry_index = ranked.entries.len();
            ranked.entries.push(ranked_entry);
            ranked
                .exact_index
                .entry(normalized_key.clone())
                .or_default()
                .push(entry_index);

            let mut seen_aliases = HashSet::new();
            for key in alias_keys {
                if seen_aliases.insert(key.clone()) {
                    ranked.alias_index.entry(key.clone()).or_default().push(entry_index);
                    for gram in char_ngrams(&key, 2) {
                        ranked.gram_index.entry(gram).or_default().push(entry_index);
                    }
                }
            }
            for gram in char_ngrams(&normalized_key, 2) {
                ranked.gram_index.entry(gram).or_default().push(entry_index);
            }
        }

        ranked
    }
}

fn boundary_tags_for_target(target: &str, corpus_stats: &CorpusStats) -> (Option<String>, Option<String>) {
    let mut words = target.split_whitespace();
    let Some(first_word) = words.next() else {
        return (None, None);
    };
    let last_word = words.last().unwrap_or(first_word);
    (
        corpus_stats.dominant_tag(first_word).map(str::to_owned),
        corpus_stats.dominant_tag(last_word).map(str::to_owned),
    )
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

fn parse_compiled_lexicon(source: &[u8]) -> Result<Vec<Entry>> {
    if source.len() < 8 || &source[..4] != COMPILED_MAGIC {
        return Err(LexiconError::Parse("invalid compiled lexicon header".to_owned()));
    }

    let entry_count = u32::from_le_bytes(source[4..8].try_into().expect("header slice has fixed width")) as usize;
    let mut offset = 8usize;
    let mut entries = Vec::with_capacity(entry_count);
    while offset < source.len() {
        let roman_end = find_nul(source, offset)
            .ok_or_else(|| LexiconError::Parse("invalid compiled lexicon payload".to_owned()))?;
        let target_start = roman_end + 1;
        let target_end = find_nul(source, target_start)
            .ok_or_else(|| LexiconError::Parse("invalid compiled lexicon payload".to_owned()))?;
        let roman = std::str::from_utf8(&source[offset..roman_end])
            .map_err(|_| LexiconError::Parse("compiled lexicon contains invalid UTF-8".to_owned()))?;
        let target = std::str::from_utf8(&source[target_start..target_end])
            .map_err(|_| LexiconError::Parse("compiled lexicon contains invalid UTF-8".to_owned()))?;
        entries.push(Entry {
            roman: roman.to_owned(),
            target: target.to_owned(),
        });
        offset = target_end + 1;
    }

    if entries.len() != entry_count {
        return Err(LexiconError::Parse("compiled lexicon entry count mismatch".to_owned()));
    }

    Ok(entries)
}

fn parse_compiled_khpos_stats(source: &[u8]) -> Result<CorpusStats> {
    if source.len() < 4 || &source[..4] != KHPOS_MAGIC {
        return Err(LexiconError::Parse("invalid compiled khPOS stats header".to_owned()));
    }

    let mut offset = 4usize;
    let word_unigrams = read_string_count_map(source, &mut offset)?;
    let word_bigrams = read_pair_count_map(source, &mut offset)?;
    let surface_unigrams = read_string_count_map(source, &mut offset)?;
    let tag_unigrams = read_string_count_map(source, &mut offset)?;
    let tag_bigrams = read_pair_count_map(source, &mut offset)?;
    let dominant_word_tags = read_dominant_tags(source, &mut offset)?;
    if offset != source.len() {
        return Err(LexiconError::Parse(
            "compiled khPOS stats has trailing bytes".to_owned(),
        ));
    }

    Ok(CorpusStats {
        word_unigrams,
        word_bigrams,
        surface_unigrams,
        tag_unigrams,
        tag_bigrams,
        dominant_word_tags,
    })
}

fn find_nul(source: &[u8], start: usize) -> Option<usize> {
    source[start..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|relative| start + relative)
}

fn read_string_count_map(source: &[u8], offset: &mut usize) -> Result<HashMap<String, u32>> {
    let count = read_u32(source, offset)? as usize;
    let mut output = HashMap::with_capacity(count);
    for _ in 0..count {
        let text = read_nul_terminated_str(source, offset)?;
        let value = read_u32(source, offset)?;
        output.insert(text, value);
    }
    Ok(output)
}

fn read_pair_count_map(source: &[u8], offset: &mut usize) -> Result<HashMap<(String, String), u32>> {
    let count = read_u32(source, offset)? as usize;
    let mut output = HashMap::with_capacity(count);
    for _ in 0..count {
        let left = read_nul_terminated_str(source, offset)?;
        let right = read_nul_terminated_str(source, offset)?;
        let value = read_u32(source, offset)?;
        output.insert((left, right), value);
    }
    Ok(output)
}

fn read_dominant_tags(source: &[u8], offset: &mut usize) -> Result<HashMap<String, DominantTag>> {
    let count = read_u32(source, offset)? as usize;
    let mut output = HashMap::with_capacity(count);
    for _ in 0..count {
        let word = read_nul_terminated_str(source, offset)?;
        let tag = read_nul_terminated_str(source, offset)?;
        let support = read_u32(source, offset)?;
        output.insert(word, DominantTag { tag, support });
    }
    Ok(output)
}

fn read_u32(source: &[u8], offset: &mut usize) -> Result<u32> {
    if source.len().saturating_sub(*offset) < 4 {
        return Err(LexiconError::Parse(
            "compiled khPOS stats payload is truncated".to_owned(),
        ));
    }
    let value = u32::from_le_bytes(source[*offset..*offset + 4].try_into().expect("slice length checked"));
    *offset += 4;
    Ok(value)
}

fn read_nul_terminated_str(source: &[u8], offset: &mut usize) -> Result<String> {
    let end = find_nul(source, *offset)
        .ok_or_else(|| LexiconError::Parse("compiled khPOS stats string is missing NUL terminator".to_owned()))?;
    let value = std::str::from_utf8(&source[*offset..end])
        .map_err(|_| LexiconError::Parse("compiled khPOS stats contains invalid UTF-8".to_owned()))?
        .to_owned();
    *offset = end + 1;
    Ok(value)
}

fn is_roman_letter(ch: char) -> bool {
    ch.is_ascii() && (ch.is_ascii_alphabetic() || ch == '_')
}

fn is_keycap_token_char(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, '!' | '"' | '#' | '$' | '%' | '&' | '\'' | '(' | ')' | '~' | '=')
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
                || *ch == '_'
                || *ch == ','
                || *ch == ' '
                || ('\u{00C0}'..='\u{00FF}').contains(ch)
                || ('\u{0621}'..='\u{064A}').contains(ch)
                || ('\u{0660}'..='\u{0669}').contains(ch)
                || ('\u{1780}'..='\u{17D2}').contains(ch)
        })
        .collect()
}

pub(crate) fn roman_search_variants(input: &str) -> Vec<String> {
    let base = normalize(input);
    if base.is_empty() {
        return Vec::new();
    }

    let mut variants = Vec::new();
    let mut seen = HashSet::new();
    push_variant(&mut variants, &mut seen, base.clone());

    let collapsed = collapse_repeated_letters(&base);
    push_variant(&mut variants, &mut seen, collapsed.clone());
    push_variant(&mut variants, &mut seen, normalize_cluster_aliases(&base));
    push_variant(&mut variants, &mut seen, normalize_cluster_aliases(&collapsed));
    push_variant(&mut variants, &mut seen, normalize_vowel_aliases(&base));
    push_variant(&mut variants, &mut seen, normalize_vowel_aliases(&collapsed));
    push_variant(&mut variants, &mut seen, normalize_final_aliases(&base));
    push_variant(&mut variants, &mut seen, normalize_final_aliases(&collapsed));
    for variant in collision_variants(&base) {
        push_variant(&mut variants, &mut seen, variant);
    }
    for variant in collision_variants(&collapsed) {
        push_variant(&mut variants, &mut seen, variant);
    }

    let final_cluster = normalize_cluster_aliases(&normalize_final_aliases(&collapsed));
    push_variant(&mut variants, &mut seen, final_cluster.clone());

    let skeleton = consonant_skeleton(&base);
    if !skeleton.is_empty() {
        push_variant(&mut variants, &mut seen, format!("sk:{skeleton}"));
    }
    let collapsed_skeleton = consonant_skeleton(&collapsed);
    if !collapsed_skeleton.is_empty() {
        push_variant(&mut variants, &mut seen, format!("sk:{collapsed_skeleton}"));
    }

    variants
}

fn push_variant(variants: &mut Vec<String>, seen: &mut HashSet<String>, variant: String) {
    if !variant.is_empty() && seen.insert(variant.clone()) {
        variants.push(variant);
    }
}

fn collapse_repeated_letters(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut previous = None::<char>;
    for ch in input.chars() {
        let allow_repeat = matches!(ch, 't' | 'n' | 'm' | 'a' | 'o');
        if previous == Some(ch) && !allow_repeat {
            continue;
        }
        output.push(ch);
        previous = Some(ch);
    }
    output
}

fn normalize_cluster_aliases(input: &str) -> String {
    let replacements = [
        ("chh", "c"),
        ("ddh", "d"),
        ("tth", "t"),
        ("kiet", "git"),
        ("kit", "git"),
        ("kh", "k"),
        ("gh", "g"),
        ("ng", "n"),
        ("nh", "n"),
        ("th", "t"),
        ("tt", "t"),
        ("dd", "d"),
        ("ph", "p"),
        ("bh", "b"),
        ("jh", "j"),
        ("ch", "c"),
    ];
    apply_replacements(input, &replacements)
}

fn normalize_vowel_aliases(input: &str) -> String {
    let replacements = [
        ("aeu", "e"),
        ("ae", "e"),
        ("ea", "e"),
        ("ei", "e"),
        ("ie", "i"),
        ("oe", "e"),
        ("eu", "e"),
        ("ue", "e"),
        ("ou", "o"),
        ("ov", "o"),
        ("aw", "o"),
        ("av", "ao"),
    ];
    apply_replacements(input, &replacements)
}

fn normalize_final_aliases(input: &str) -> String {
    for suffix in ["aors", "aor", "aos", "aoh", "ors", "or", "os", "oh", "rs"] {
        if let Some(stem) = input.strip_suffix(suffix) {
            return format!("{stem}aoh");
        }
    }
    input.to_owned()
}

fn collision_variants(input: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let mut seen = HashSet::new();
    let pairs = [
        ("ch", "j"),
        ("j", "ch"),
        ("bb", "p"),
        ("p", "bb"),
        ("tt", "t"),
        ("t", "tt"),
        ("ue", "eu"),
        ("eu", "ue"),
    ];

    for (from, to) in pairs {
        for variant in replace_once_variants(input, from, to) {
            if variant != input && seen.insert(variant.clone()) {
                variants.push(variant);
            }
        }
    }

    variants
}

fn replace_once_variants(input: &str, from: &str, to: &str) -> Vec<String> {
    if from.is_empty() || !input.contains(from) {
        return Vec::new();
    }
    let mut variants = Vec::new();
    let mut search_from = 0usize;
    while let Some(found) = input[search_from..].find(from) {
        let start = search_from + found;
        let end = start + from.len();
        let mut candidate = String::with_capacity(input.len() + to.len().saturating_sub(from.len()));
        candidate.push_str(&input[..start]);
        candidate.push_str(to);
        candidate.push_str(&input[end..]);
        variants.push(candidate);
        search_from = start + 1;
    }
    variants
}

fn consonant_skeleton(input: &str) -> String {
    let mut output = String::new();
    for ch in normalize_cluster_aliases(input).chars() {
        if !matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u' | 'y') {
            if output.chars().last() != Some(ch) {
                output.push(ch);
            }
        }
    }
    output
}

fn apply_replacements(input: &str, replacements: &[(&str, &str)]) -> String {
    let mut output = input.to_owned();
    for (from, to) in replacements {
        output = output.replace(from, to);
    }
    output
}

pub(crate) fn char_ngrams(input: &str, size: usize) -> Vec<String> {
    let chars = input.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Vec::new();
    }
    if chars.len() <= size {
        return vec![chars.iter().collect()];
    }

    let mut grams = Vec::with_capacity(chars.len().saturating_sub(size) + 1);
    for start in 0..=chars.len().saturating_sub(size) {
        grams.push(chars[start..start + size].iter().collect());
    }
    grams
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
    const DEFAULT_DATA_TSV: &str = include_str!("../data/roman_lookup.tsv");

    #[test]
    fn loads_embedded_data() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let expected_entries = DEFAULT_DATA_TSV.lines().filter(|line| !line.is_empty()).count();
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
    fn compiled_lexicon_matches_tsv_source() {
        let compiled_entries = parse_compiled_lexicon(DEFAULT_COMPILED_DATA).unwrap();
        let tsv_entries = parse_tsv(DEFAULT_DATA_TSV).unwrap();
        assert_eq!(compiled_entries, tsv_entries);
    }

    #[test]
    fn loads_embedded_khpos_stats() {
        let stats = parse_compiled_khpos_stats(DEFAULT_COMPILED_KHPOS_STATS).unwrap();
        assert!(stats.word_unigrams.get("ខ្ញុំ").copied().unwrap_or(0) > 0);
        assert!(stats.surface_unigrams.get("ខ្ញុំ").copied().unwrap_or(0) > 0);
        assert!(stats.surface_unigrams.get("ខ្ញុំទៅ").copied().unwrap_or(0) > 0);
        assert!(
            stats
                .word_bigrams
                .get(&(String::from("ខ្ញុំ"), String::from("ទៅ")))
                .copied()
                .unwrap_or(0)
                > 0
        );
        assert_eq!(stats.dominant_tag("ខ្ញុំ"), Some("PRO"));
        assert!(stats
            .dominant_word_tags
            .get("ខ្ញុំ")
            .map(|entry| entry.support > 0)
            .unwrap_or(false));
    }

    #[test]
    fn ranked_lexicon_assigns_boundary_tags_for_phrase_entries() {
        let stats = parse_compiled_khpos_stats(DEFAULT_COMPILED_KHPOS_STATS).unwrap();
        let ranked = RankedLexicon::from_entries(
            &[
                Entry {
                    roman: "khnhom".to_owned(),
                    target: "ខ្ញុំ".to_owned(),
                },
                Entry {
                    roman: "khnhomttov".to_owned(),
                    target: "ខ្ញុំ ទៅ".to_owned(),
                },
            ],
            &stats,
        );

        let single = ranked.entries.iter().find(|entry| entry.target == "ខ្ញុំ").unwrap();
        assert_eq!(single.first_tag.as_deref(), stats.dominant_tag("ខ្ញុំ"));
        assert_eq!(single.last_tag.as_deref(), stats.dominant_tag("ខ្ញុំ"));

        let phrase = ranked.entries.iter().find(|entry| entry.target == "ខ្ញុំ ទៅ").unwrap();
        assert_eq!(phrase.first_tag.as_deref(), stats.dominant_tag("ខ្ញុំ"));
        assert_eq!(phrase.last_tag.as_deref(), stats.dominant_tag("ទៅ"));
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
            vec!["ទៅ", "តើ", "វេរ", "ថេរ", "ទេរ", "ធ្វើ", "ដំណើរ", "សរសើរ", "ខ្វេរ", "ទង្វើ"]
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
            vec!["ទេ", "តែ", "តើ", "តិះ", "តេត", "តេន", "តិច", "ធ្វើ", "ទន្លេ", "ផ្ទេរ"]
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
    fn supports_top_row_keycap_suggestions() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let history = HashMap::new();
        let expectations = [
            ("1", "១"),
            ("!", "!"),
            ("2", "២"),
            ("\"", "ៗ"),
            ("3", "៣"),
            ("#", "\""),
            ("4", "៤"),
            ("$", "៛"),
            ("5", "៥"),
            ("%", "%"),
            ("6", "៦"),
            ("&", "៍"),
            ("7", "៧"),
            ("'", "័"),
            ("8", "៨"),
            ("(", "៏"),
            ("9", "៩"),
            (")", "("),
            ("0", "០"),
            ("~", ")"),
            ("=", "៌"),
        ];

        for (input, expected) in expectations {
            assert_eq!(transliterator.suggest(input, &history), vec![expected.to_owned()]);
        }
    }

    #[test]
    fn maps_multi_digit_queries_to_khmer_digits() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let history = HashMap::new();
        assert_eq!(transliterator.suggest("21212", &history), vec!["២១២១២".to_owned()]);
        assert_eq!(transliterator.suggest("09876", &history), vec!["០៩៨៧៦".to_owned()]);
    }

    #[test]
    fn normalize_preserves_underscore_for_disambiguation() {
        assert_eq!(normalize("b_eh"), "b_eh");
        assert_eq!(normalize("B_EH"), "b_eh");
    }

    #[test]
    fn token_bounds_supports_keycap_sequences() {
        assert_eq!(Transliterator::token_bounds("21212", 5, false), 0..5);
        assert_eq!(Transliterator::token_bounds("bong 21212", 10, false), 5..10);
        assert_eq!(Transliterator::token_bounds("bong!", 5, false), 4..5);
        assert_eq!(Transliterator::token_bounds("bong 2", 6, false), 5..6);
    }

    #[test]
    fn token_bounds_treats_underscore_as_part_of_roman_token() {
        assert_eq!(Transliterator::token_bounds("b_eh", 4, false), 0..4);
        assert_eq!(Transliterator::token_bounds("foo b_eh", 8, false), 4..8);
    }

    #[test]
    fn suggest_distinguishes_entries_with_underscore() {
        let fixture = "beh\tបេះ\nb_eh\tប៊ិះ\n";
        let transliterator = Transliterator::from_tsv_str(fixture).unwrap();
        let history = HashMap::new();
        assert_eq!(
            transliterator.suggest("b_eh", &history).first().map(String::as_str),
            Some("ប៊ិះ")
        );
    }

    #[test]
    fn exact_match_targets_only_return_exact_roman_mappings() {
        let fixture = "barko\tបាកូ\nbark\tប៉ប្រោក\n";
        let transliterator = Transliterator::from_tsv_str(fixture).unwrap();
        assert_eq!(transliterator.exact_match_targets("barko"), vec!["បាកូ".to_owned()]);
        assert_eq!(transliterator.exact_match_targets("BARK"), vec!["ប៉ប្រោក".to_owned()]);
    }

    #[test]
    fn best_prefix_consumption_prefers_longest_matching_target_roman() {
        let transliterator = Transliterator::from_default_data().unwrap();
        assert_eq!(
            transliterator.best_prefix_consumption("cheamnouslaor", "ជា").as_deref(),
            Some("chea")
        );
    }

    #[test]
    fn shadow_observation_exposes_bounded_decoder_results() {
        let transliterator = Transliterator::from_default_data_with_config(
            DecoderConfig::default().with_mode(crate::decoder::DecoderMode::Shadow),
        )
        .unwrap();

        let observation = transliterator.shadow_observation("jea", &HashMap::new());
        assert_ne!(observation.mismatch, crate::decoder::ShadowMismatch::WfstUnavailable);
        assert_eq!(observation.wfst_top.as_deref(), Some("ជា"));
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

    #[test]
    fn composes_exact_phrase_chunks_before_fuzzy_whole_token_matches() {
        let transliterator = Transliterator::from_default_data().unwrap();
        assert_eq!(
            transliterator
                .suggest("khnhomttov", &HashMap::new())
                .first()
                .map(String::as_str),
            Some("ខ្ញុំ ទៅ")
        );
    }

    #[test]
    fn search_variants_cover_repeated_letters_and_soft_finals() {
        let repeated = roman_search_variants("knhhom");
        assert!(repeated.iter().any(|variant| variant == "knhom"));

        let finals = roman_search_variants("sronors");
        assert!(finals.iter().any(|variant| variant == "sronaoh"));
    }

    #[test]
    fn search_variants_cover_ue_eu_aliases() {
        let eu = roman_search_variants("heub");
        let ue = roman_search_variants("hueb");
        assert!(eu.iter().any(|variant| variant == "heb"));
        assert!(ue.iter().any(|variant| variant == "heb"));
    }

    #[test]
    fn search_variants_cover_common_rule_collisions() {
        let j = roman_search_variants("jea");
        let ch = roman_search_variants("chea");
        assert!(j.iter().any(|variant| variant == "chea"));
        assert!(ch.iter().any(|variant| variant == "jea"));

        let p = roman_search_variants("pa");
        let bb = roman_search_variants("bba");
        assert!(p.iter().any(|variant| variant == "bba"));
        assert!(bb.iter().any(|variant| variant == "pa"));

        let t = roman_search_variants("ta");
        let tt = roman_search_variants("tta");
        assert!(t.iter().any(|variant| variant == "tta"));
        assert!(tt.iter().any(|variant| variant == "ta"));
    }

    #[test]
    fn suggest_recovers_targets_from_common_collision_typos() {
        let fixture = "chea\tជា\nbbong\tបង\nttae\tតែ\nhueb\tហួប\n";
        let transliterator = Transliterator::from_tsv_str(fixture).unwrap();
        let history = HashMap::new();

        assert_eq!(
            transliterator.suggest("jea", &history).first().map(String::as_str),
            Some("ជា")
        );
        assert_eq!(
            transliterator.suggest("pong", &history).first().map(String::as_str),
            Some("បង")
        );
        assert_eq!(
            transliterator.suggest("tae", &history).first().map(String::as_str),
            Some("តែ")
        );
        assert_eq!(
            transliterator.suggest("heub", &history).first().map(String::as_str),
            Some("ហួប")
        );
    }

    #[test]
    fn search_variants_cover_kit_git_aliases() {
        let variants = roman_search_variants("kit");
        assert!(variants.iter().any(|variant| variant == "git"));
    }
}
