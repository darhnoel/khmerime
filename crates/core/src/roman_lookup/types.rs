use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::decoder::DecoderManager;

use super::search_index::SearchIndex;

#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
pub(super) const DEFAULT_COMPILED_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/roman_lookup.lexicon.bin"));
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
pub(super) const DEFAULT_COMPILED_KHPOS_STATS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/khpos.stats.bin"));
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
pub(super) const DEFAULT_COMPILED_NEXT_WORD_STATS: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/next_word.stats.bin"));
pub(super) const COMPILED_MAGIC: &[u8; 4] = b"RLX1";
pub(super) const KHPOS_MAGIC: &[u8; 4] = b"KPS1";
pub(super) const NEXT_WORD_MAGIC: &[u8; 4] = b"NWS1";
pub(super) const MAX_SUGGESTIONS: usize = 15;
pub(super) const MAX_MATCHES: usize = 20;
pub(super) const NEXT_WORD_BACKFILL_POOL: usize = 128;
pub(super) const KEYCAP_SUGGESTIONS: [(&str, &str); 21] = [
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

pub(super) fn khmer_digit(ch: char) -> Option<char> {
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

pub(super) const PRIORITY_SEEDS: [(&str, &str); 39] = [
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

#[derive(Clone, Debug, Default)]
pub(crate) struct NextWordStats {
    pub unigrams: HashMap<String, u32>,
    pub bigrams: HashMap<String, Vec<(String, u32)>>,
    pub ranked_unigrams: Vec<(String, u32)>,
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
    pub(super) entries: Vec<Entry>,
    pub(super) by_roman: HashMap<String, Vec<String>>,
    pub(super) by_normalized: HashMap<String, Vec<String>>,
    pub(super) by_target: HashMap<String, Vec<String>>,
    pub(super) roman_normalized: HashMap<String, String>,
    pub(super) roman_prefix_index: HashMap<String, Vec<String>>,
    pub(super) index: SearchIndex,
    pub(super) ranked: RankedLexicon,
    pub(super) next_word: NextWordStats,
    pub(super) next_word_max_context_chars: usize,
}

pub struct Transliterator {
    pub(super) legacy: Arc<LegacyData>,
    pub(super) decoder: DecoderManager,
}

pub(super) struct LegacyLookupMaps {
    pub by_roman: HashMap<String, Vec<String>>,
    pub by_normalized: HashMap<String, Vec<String>>,
    pub by_target: HashMap<String, Vec<String>>,
    pub roman_normalized: HashMap<String, String>,
    pub roman_prefix_index: HashMap<String, Vec<String>>,
    pub roman_keys: Vec<String>,
}
