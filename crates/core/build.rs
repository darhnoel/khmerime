use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DATA_PATHS_CONFIG_PATH: &str = "../../config/data_paths.toml";
const DEFAULT_TSV_PATH: &str = "../../data/roman_lookup.tsv";
const DEFAULT_CSV_PATH: &str = "../../data/roman_lookup.csv";
const DEFAULT_ADDITIONAL_CSV_PATH: &str = "../../data/most-common-en-kh.csv";
const DEFAULT_KHPOS_TRAIN_PATH: &str = "../../data/khPOS/corpus-draft-ver-1.0/data/after-replace/train.all";
const DEFAULT_KHPOS_TAG_PATH: &str = "../../data/khPOS/corpus-draft-ver-1.0/data/after-replace/train.all.tag";
const DEFAULT_MOBILE_KEYBOARD_1GRAM_PATH: &str =
    "../../data/khmerlang-mobile-keyboard-data/keyboard-data/extracted/mobile-keyboard-data-1gram.csv";
const DEFAULT_MOBILE_KEYBOARD_2GRAM_PATH: &str =
    "../../data/khmerlang-mobile-keyboard-data/keyboard-data/extracted/mobile-keyboard-data-2gram.csv";

// These magic bytes version the compact binary blobs embedded into the Rust
// binary. Runtime parsers use them to reject stale or mismatched generated data
// before interpreting offsets and counts.
const MAGIC: &[u8; 4] = b"RLX2";
const KHPOS_MAGIC: &[u8; 4] = b"KPS1";
const NEXT_WORD_MAGIC: &[u8; 4] = b"NWS1";
const MAX_JOINED_SURFACE_TOKENS: usize = 4;

#[path = "src/roman_lookup/dictionary_image_format.rs"]
mod dictionary_image_format;
#[allow(dead_code)]
#[path = "src/roman_lookup/normalization.rs"]
mod normalization;

use dictionary_image_format::*;
use normalization::{normalize, roman_search_variants};

#[derive(Clone, Copy)]
enum LexiconSourceFormat {
    Csv,
    Tsv,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BuildLexiconEntry {
    roman: String,
    target: String,
    frequency: u32,
    frequency_lang: String,
}

impl BuildLexiconEntry {
    fn new(roman: String, target: String, frequency: u32, frequency_lang: String) -> Self {
        Self {
            roman,
            target,
            frequency,
            frequency_lang,
        }
    }

    fn default_frequency(roman: String, target: String) -> Self {
        Self::new(roman, target, 1, "km".to_owned())
    }
}

#[derive(Clone, Debug)]
struct BuildDataConfig {
    paths: BuildDataPaths,
    build: BuildDataBuildOptions,
    runtime: BuildRuntimeOptions,
}

impl Default for BuildDataConfig {
    fn default() -> Self {
        Self {
            paths: BuildDataPaths::default(),
            build: BuildDataBuildOptions::default(),
            runtime: BuildRuntimeOptions::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LegacyFuzzyIndexKind {
    Ngram,
    SymSpell,
}

impl LegacyFuzzyIndexKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ngram => "ngram",
            Self::SymSpell => "symspell",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BuildRuntimeOptions {
    legacy_fuzzy_index: LegacyFuzzyIndexKind,
}

impl Default for BuildRuntimeOptions {
    fn default() -> Self {
        Self {
            legacy_fuzzy_index: LegacyFuzzyIndexKind::Ngram,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BuildDataBuildOptions {
    khpos_surface_min_count: u32,
    khpos_surface_top_n: Option<usize>,
}

impl Default for BuildDataBuildOptions {
    fn default() -> Self {
        Self {
            khpos_surface_min_count: 1,
            khpos_surface_top_n: None,
        }
    }
}

impl BuildDataBuildOptions {
    fn with_env_overrides(mut self) -> Result<Self, String> {
        if let Some(value) = env::var("KHPOS_SURFACE_MIN_COUNT")
            .ok()
            .filter(|value| !value.is_empty())
        {
            self.khpos_surface_min_count = value
                .parse::<u32>()
                .map_err(|_| format!("KHPOS_SURFACE_MIN_COUNT must be a u32, got '{value}'"))?;
        }
        if let Some(value) = env::var("KHPOS_SURFACE_TOP_N").ok().filter(|value| !value.is_empty()) {
            let limit = value
                .parse::<usize>()
                .map_err(|_| format!("KHPOS_SURFACE_TOP_N must be a usize, got '{value}'"))?;
            self.khpos_surface_top_n = (limit > 0).then_some(limit);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug)]
struct BuildDataPaths {
    lexicon_csv: String,
    lexicon_tsv: String,
    khpos_train: String,
    khpos_tag: String,
    mobile_keyboard_1gram: String,
    mobile_keyboard_2gram: String,
}

impl Default for BuildDataPaths {
    fn default() -> Self {
        Self {
            lexicon_csv: DEFAULT_CSV_PATH.to_owned(),
            lexicon_tsv: DEFAULT_TSV_PATH.to_owned(),
            khpos_train: DEFAULT_KHPOS_TRAIN_PATH.to_owned(),
            khpos_tag: DEFAULT_KHPOS_TAG_PATH.to_owned(),
            mobile_keyboard_1gram: DEFAULT_MOBILE_KEYBOARD_1GRAM_PATH.to_owned(),
            mobile_keyboard_2gram: DEFAULT_MOBILE_KEYBOARD_2GRAM_PATH.to_owned(),
        }
    }
}

fn normalize_workspace_path(path: &str) -> String {
    if Path::new(path).is_absolute() || Path::new(path).exists() {
        return path.to_owned();
    }
    let candidate = format!("../../{path}");
    if Path::new(&candidate).exists() {
        candidate
    } else {
        path.to_owned()
    }
}

fn main() {
    let data_config = load_data_config_from_config();
    let mut data_paths = data_config.paths;
    let khpos_build_options = data_config
        .build
        .with_env_overrides()
        .expect("data build environment overrides must be valid");
    data_paths.lexicon_csv = normalize_workspace_path(&data_paths.lexicon_csv);
    data_paths.lexicon_tsv = normalize_workspace_path(&data_paths.lexicon_tsv);
    data_paths.khpos_train = normalize_workspace_path(&data_paths.khpos_train);
    data_paths.khpos_tag = normalize_workspace_path(&data_paths.khpos_tag);
    data_paths.mobile_keyboard_1gram = normalize_workspace_path(&data_paths.mobile_keyboard_1gram);
    data_paths.mobile_keyboard_2gram = normalize_workspace_path(&data_paths.mobile_keyboard_2gram);
    let additional_lexicon_csv = normalize_workspace_path(DEFAULT_ADDITIONAL_CSV_PATH);
    println!("cargo:rerun-if-changed={}", data_paths.lexicon_csv);
    println!("cargo:rerun-if-changed={}", data_paths.lexicon_tsv);
    println!("cargo:rerun-if-changed={additional_lexicon_csv}");
    println!("cargo:rerun-if-changed={}", data_paths.khpos_train);
    println!("cargo:rerun-if-changed={}", data_paths.khpos_tag);
    println!("cargo:rerun-if-changed={}", data_paths.mobile_keyboard_1gram);
    println!("cargo:rerun-if-changed={}", data_paths.mobile_keyboard_2gram);
    println!("cargo:rerun-if-env-changed=KHMERIME_WARN_MISSING_OPTIONAL_DATA");
    println!("cargo:rerun-if-env-changed=KHPOS_SURFACE_MIN_COUNT");
    println!("cargo:rerun-if-env-changed=KHPOS_SURFACE_TOP_N");
    println!("cargo:rerun-if-env-changed=KHMERIME_WARN_MISSING_OPTIONAL_DATA");

    // Set KHMERIME_WARN_MISSING_OPTIONAL_DATA=1 when auditing data-path
    // configuration and you want missing optional files to be visible as Cargo
    // warnings.
    let warn_missing_optional_data = env::var_os("KHMERIME_WARN_MISSING_OPTIONAL_DATA").is_some();

    // The checked-in CSV is canonical, but the TSV fallback keeps older local
    // worktrees usable while data migrations are in flight.
    let (source, source_format) = match fs::read_to_string(&data_paths.lexicon_csv) {
        Ok(source) => (source, LexiconSourceFormat::Csv),
        Err(_) => (
            fs::read_to_string(&data_paths.lexicon_tsv).expect("default lexicon CSV/TSV must be readable"),
            LexiconSourceFormat::Tsv,
        ),
    };
    let mut entries = parse_lexicon_entries(&source, source_format).expect("default lexicon CSV/TSV must compile");
    let additional_source =
        fs::read_to_string(&additional_lexicon_csv).expect("additional most-common English-Khmer CSV must be readable");
    entries.extend(parse_additional_csv_entries(&additional_source));
    // khPOS corpus files improve decoding quality, but they are large and
    // gitignored. CI runners won't have them unless the step runner downloads
    // them separately. Treat them as optional like mobile keyboard n-grams.
    let khpos_train = read_optional_source(
        &data_paths.khpos_train,
        "khPOS after-replace train corpus",
        warn_missing_optional_data,
    );
    let khpos_tags = read_optional_source(
        &data_paths.khpos_tag,
        "khPOS after-replace tag corpus",
        warn_missing_optional_data,
    );
    let compiled_khpos = compile_khpos_stats(&khpos_train, &khpos_tags, khpos_build_options)
        .expect("khPOS after-replace corpus must compile");
    let compiled_dictionary_image = compile_dictionary_image(&entries, Some(&compiled_khpos.frequency_stats))
        .expect("default dictionary image must compile");
    let compiled = compile_lexicon_entries(entries).expect("default lexicon entries must compile");
    let mobile_keyboard_1gram = read_optional_source(
        &data_paths.mobile_keyboard_1gram,
        "mobile keyboard 1-gram",
        warn_missing_optional_data,
    );
    let mobile_keyboard_2gram = read_optional_source(
        &data_paths.mobile_keyboard_2gram,
        "mobile keyboard 2-gram",
        warn_missing_optional_data,
    );
    let compiled_next_word = compile_next_word_stats(&mobile_keyboard_1gram, &mobile_keyboard_2gram)
        .expect("mobile keyboard n-gram data must compile");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let output_path = out_dir.join("roman_lookup.lexicon.bin");
    fs::write(&output_path, compiled).expect("compiled lexicon must be written");
    let dictionary_image_output_path = out_dir.join("khmerime.dictionary_image.bin");
    fs::write(&dictionary_image_output_path, compiled_dictionary_image)
        .expect("compiled dictionary image must be written");
    let khpos_output_path = out_dir.join("khpos.stats.bin");
    fs::write(&khpos_output_path, compiled_khpos.bytes).expect("compiled khPOS stats must be written");
    let next_word_output_path = out_dir.join("next_word.stats.bin");
    fs::write(&next_word_output_path, compiled_next_word).expect("compiled next-word stats must be written");
    let search_index_config_path = out_dir.join("search_index_config.rs");
    fs::write(
        &search_index_config_path,
        search_index_config_source(data_config.runtime),
    )
    .expect("search index config must be written");

    // When building for wasm32 with the fetch-data feature, copy the compiled
    // binary blobs into assets/data/ so Dioxus serves them as static files.
    let target = env::var("TARGET").unwrap_or_default();
    let fetch_data = env::var("CARGO_FEATURE_FETCH_DATA").is_ok();
    if target.starts_with("wasm32") && fetch_data {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
        let assets_data = PathBuf::from(manifest_dir).join("../../assets/data");
        fs::create_dir_all(&assets_data).expect("assets/data dir must be creatable");
        fs::copy(&output_path, assets_data.join("roman_lookup.lexicon.bin"))
            .expect("lexicon bin must copy to assets/data");
        fs::copy(
            &dictionary_image_output_path,
            assets_data.join("khmerime.dictionary_image.bin"),
        )
        .expect("dictionary image must copy to assets/data");
        fs::copy(&khpos_output_path, assets_data.join("khpos.stats.bin")).expect("khpos bin must copy to assets/data");
        fs::copy(&next_word_output_path, assets_data.join("next_word.stats.bin"))
            .expect("next-word bin must copy to assets/data");
    }
}

fn load_data_config_from_config() -> BuildDataConfig {
    println!("cargo:rerun-if-changed={DATA_PATHS_CONFIG_PATH}");
    let mut config = BuildDataConfig::default();
    let Ok(source) = fs::read_to_string(DATA_PATHS_CONFIG_PATH) else {
        return config;
    };
    if let Err(error) = apply_data_config(&source, &mut config) {
        panic!("{DATA_PATHS_CONFIG_PATH} parse failed: {error}");
    }
    config
}

fn read_optional_source(path: &str, label: &str, warn_when_missing: bool) -> String {
    match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            if warn_when_missing {
                println!("cargo:warning={label} data not found at {path}: {error}; compiling empty optional dataset");
            }
            // Empty optional data compiles to a valid empty stats blob so every
            // target can build without vendoring large experimental datasets.
            String::new()
        }
    }
}

fn apply_data_config(source: &str, config: &mut BuildDataConfig) -> Result<(), String> {
    let mut section = "";
    for (line_no, raw_line) in source.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim();
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!("invalid config format on line {}", line_no + 1));
        };
        let key = raw_key.trim();
        match section {
            "data_paths" => apply_data_paths_config_value(key, raw_value.trim(), line_no + 1, &mut config.paths)?,
            "data_build" => apply_data_build_config_value(key, raw_value.trim(), line_no + 1, &mut config.build)?,
            "runtime" => apply_runtime_config_value(key, raw_value.trim(), line_no + 1, &mut config.runtime)?,
            _ => continue,
        }
    }
    Ok(())
}

fn apply_data_paths_config_value(
    key: &str,
    raw_value: &str,
    line_no: usize,
    paths: &mut BuildDataPaths,
) -> Result<(), String> {
    let value = parse_data_path_value(raw_value, line_no)?;
    if value.is_empty() {
        return Err(format!("empty value for '{}' on line {}", key, line_no));
    }
    match key {
        "lexicon_csv" => paths.lexicon_csv = value,
        "lexicon_tsv" => paths.lexicon_tsv = value,
        "khpos_train" => paths.khpos_train = value,
        "khpos_tag" => paths.khpos_tag = value,
        "mobile_keyboard_1gram" => paths.mobile_keyboard_1gram = value,
        "mobile_keyboard_2gram" => paths.mobile_keyboard_2gram = value,
        _ => return Err(format!("unknown key '{}' in [data_paths] on line {}", key, line_no)),
    }
    Ok(())
}

fn apply_data_build_config_value(
    key: &str,
    raw_value: &str,
    line_no: usize,
    build: &mut BuildDataBuildOptions,
) -> Result<(), String> {
    match key {
        "khpos_surface_min_count" => {
            build.khpos_surface_min_count = parse_u32_config_value(raw_value, key, line_no)?;
        }
        "khpos_surface_top_n" => {
            let limit = parse_usize_config_value(raw_value, key, line_no)?;
            build.khpos_surface_top_n = (limit > 0).then_some(limit);
        }
        _ => return Err(format!("unknown key '{}' in [data_build] on line {}", key, line_no)),
    }
    Ok(())
}

fn apply_runtime_config_value(
    key: &str,
    raw_value: &str,
    line_no: usize,
    runtime: &mut BuildRuntimeOptions,
) -> Result<(), String> {
    match key {
        "legacy_fuzzy_index" => {
            runtime.legacy_fuzzy_index = parse_legacy_fuzzy_index(raw_value, line_no)?;
        }
        _ => return Err(format!("unknown key '{}' in [runtime] on line {}", key, line_no)),
    }
    Ok(())
}

fn parse_legacy_fuzzy_index(raw: &str, line_no: usize) -> Result<LegacyFuzzyIndexKind, String> {
    let value = parse_data_path_value(raw, line_no)?;
    match value.as_str() {
        "ngram" => Ok(LegacyFuzzyIndexKind::Ngram),
        "symspell" => Ok(LegacyFuzzyIndexKind::SymSpell),
        _ => Err(format!(
            "invalid legacy_fuzzy_index on line {}: '{}'; expected 'ngram' or 'symspell'",
            line_no, value
        )),
    }
}

fn parse_data_path_value(raw: &str, line_no: usize) -> Result<String, String> {
    if raw.starts_with('"') {
        if !raw.ends_with('"') || raw.len() < 2 {
            return Err(format!("unterminated quoted value on line {}", line_no));
        }
        let content = &raw[1..raw.len() - 1];
        return Ok(content.replace("\\\\", "\\").replace("\\\"", "\""));
    }
    Ok(raw.to_owned())
}

fn parse_u32_config_value(raw: &str, key: &str, line_no: usize) -> Result<u32, String> {
    raw.parse::<u32>()
        .map_err(|_| format!("invalid u32 for '{}' on line {}: '{}'", key, line_no, raw))
}

fn parse_usize_config_value(raw: &str, key: &str, line_no: usize) -> Result<usize, String> {
    raw.parse::<usize>()
        .map_err(|_| format!("invalid usize for '{}' on line {}: '{}'", key, line_no, raw))
}

fn search_index_config_source(runtime: BuildRuntimeOptions) -> String {
    format!(
        "pub(super) const DEFAULT_LEGACY_FUZZY_INDEX: &str = {:?};\n",
        runtime.legacy_fuzzy_index.as_str()
    )
}

fn compile_next_word_stats(unigram_source: &str, bigram_source: &str) -> Result<Vec<u8>, String> {
    let mut unigram_counts = HashMap::<String, u32>::new();
    let mut bigram_counts = HashMap::<(String, String), u32>::new();

    parse_unigram_rows(unigram_source, &mut unigram_counts)?;
    parse_bigram_rows(bigram_source, &mut bigram_counts)?;

    let mut output = Vec::new();
    output.extend_from_slice(NEXT_WORD_MAGIC);
    write_string_count_map(&mut output, &unigram_counts)?;
    write_pair_count_map(&mut output, &bigram_counts)?;
    Ok(output)
}

fn parse_unigram_rows(source: &str, unigram_counts: &mut HashMap<String, u32>) -> Result<(), String> {
    let mut first_row = true;
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = parse_csv_fields(line, line_no + 1)?;
        if fields.len() < 2 {
            continue;
        }
        if line_no == 0 {
            fields[0] = fields[0].trim_start_matches('\u{feff}').to_owned();
        }
        if first_row
            && fields[0].trim().eq_ignore_ascii_case("word")
            && fields[1].trim().eq_ignore_ascii_case("frequency")
        {
            first_row = false;
            continue;
        }
        first_row = false;

        let raw_word = fields[0].trim();
        let Ok(raw_frequency) = fields[1].trim().parse::<u32>() else {
            continue;
        };
        if raw_frequency == 0 {
            continue;
        }

        let word = normalize_next_word_token(raw_word);
        if !is_khmer_token(&word) {
            continue;
        }
        *unigram_counts.entry(word).or_default() += raw_frequency;
    }
    Ok(())
}

fn parse_bigram_rows(source: &str, bigram_counts: &mut HashMap<(String, String), u32>) -> Result<(), String> {
    let mut first_row = true;
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = parse_csv_fields(line, line_no + 1)?;
        if fields.len() < 2 {
            continue;
        }
        if line_no == 0 {
            fields[0] = fields[0].trim_start_matches('\u{feff}').to_owned();
        }
        if first_row
            && fields[0].trim().eq_ignore_ascii_case("word")
            && fields[1].trim().eq_ignore_ascii_case("frequency")
        {
            first_row = false;
            continue;
        }
        first_row = false;

        let raw_phrase = fields[0].trim();
        let Ok(raw_frequency) = fields[1].trim().parse::<u32>() else {
            continue;
        };
        if raw_frequency == 0 {
            continue;
        }

        let parts = raw_phrase.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 2 {
            continue;
        }
        let left = normalize_next_word_token(parts[0]);
        let right = normalize_next_word_token(parts[1]);
        if !is_valid_left_context(&left) || !is_khmer_token(&right) {
            continue;
        }
        *bigram_counts.entry((left, right)).or_default() += raw_frequency;
    }
    Ok(())
}

fn normalize_next_word_token(token: &str) -> String {
    token.trim().chars().filter(|ch| *ch != '\u{200b}').collect::<String>()
}

fn is_valid_left_context(token: &str) -> bool {
    matches!(token, "<s>" | "<num>" | "<oth>" | "<unk>") || is_khmer_token(token)
}

fn is_khmer_token(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|ch| {
            ('\u{1780}'..='\u{17ff}').contains(&ch)
                || ('\u{19e0}'..='\u{19ff}').contains(&ch)
                || ch == '\u{200c}'
                || ch == '\u{200d}'
        })
}

fn compile_lexicon_entries(entries: Vec<BuildLexiconEntry>) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    output.extend_from_slice(MAGIC);

    let entry_count = entries.len() as u32;
    output.extend_from_slice(&entry_count.to_le_bytes());

    for (line_no, entry) in entries.into_iter().enumerate() {
        if entry.roman.contains('\0') || entry.target.contains('\0') || entry.frequency_lang.contains('\0') {
            return Err(format!("NUL byte is not supported on line {}", line_no + 1));
        }
        output.extend_from_slice(entry.roman.as_bytes());
        output.push(0);
        output.extend_from_slice(entry.target.as_bytes());
        output.push(0);
        write_u32(&mut output, entry.frequency);
        output.extend_from_slice(entry.frequency_lang.as_bytes());
        output.push(0);
    }

    Ok(output)
}

#[derive(Default)]
struct DictionaryImageInterner {
    ids: HashMap<String, u32>,
    strings: Vec<String>,
}

impl DictionaryImageInterner {
    fn intern(&mut self, value: &str) -> Result<u32, String> {
        if let Some(id) = self.ids.get(value) {
            return Ok(*id);
        }
        let id = u32::try_from(self.strings.len())
            .map_err(|_| "dictionary image string table exceeded u32 ids".to_owned())?;
        self.ids.insert(value.to_owned(), id);
        self.strings.push(value.to_owned());
        Ok(id)
    }
}

struct DictionaryImageEntryRecord {
    target_id: u32,
    canonical_roman_id: u32,
    normalized_key_id: u32,
    frequency: u32,
    frequency_lang_id: u32,
    first_tag_id: u32,
    last_tag_id: u32,
}

struct CompiledKhposStats {
    bytes: Vec<u8>,
    frequency_stats: BuildCorpusFrequencyStats,
}

#[derive(Default)]
struct BuildCorpusFrequencyStats {
    word_unigrams: HashMap<String, u32>,
    surface_unigrams: HashMap<String, u32>,
    // word -> dominant POS tag, mirroring the runtime CorpusStats.dominant_word_tags
    // so the dictionary image can carry per-entry boundary tags.
    dominant_word_tags: HashMap<String, String>,
}

fn compile_dictionary_image(
    entries: &[BuildLexiconEntry],
    corpus_stats: Option<&BuildCorpusFrequencyStats>,
) -> Result<Vec<u8>, String> {
    let mut interner = DictionaryImageInterner::default();
    let mut image_entries = Vec::<DictionaryImageEntryRecord>::with_capacity(entries.len());
    // Per-entry alias-key string ids, parallel to image_entries. Lets the ranked
    // entry table's alias_keys (and thus score_forms) be served from the image.
    let mut entry_alias_ids = Vec::<Vec<u32>>::with_capacity(entries.len());
    let mut exact_index = BTreeMap::<String, Vec<u32>>::new();
    let mut alias_index = BTreeMap::<String, Vec<u32>>::new();
    let mut gram_index = BTreeMap::<String, Vec<u32>>::new();
    let mut target_frequency = HashMap::<(String, String), u32>::new();

    for entry in entries {
        if entry.roman.contains('\0') || entry.target.contains('\0') || entry.frequency_lang.contains('\0') {
            return Err("NUL byte is not supported in dictionary image entries".to_owned());
        }
        let normalized_key = normalize(&entry.roman);
        if normalized_key.is_empty() {
            continue;
        }

        let target_id = interner.intern(&entry.target)?;
        let canonical_roman_id = interner.intern(&entry.roman)?;
        let normalized_key_id = interner.intern(&normalized_key)?;
        let frequency_lang_id = interner.intern(&entry.frequency_lang)?;
        // Mirror RankedLexicon's effective frequency: max of the entry frequency and
        // the corpus surface/word unigram count, so the image entry table matches the
        // heap ranked table built with the same corpus stats.
        let corpus_frequency = corpus_stats
            .and_then(|stats| {
                stats
                    .surface_unigrams
                    .get(&entry.target)
                    .or_else(|| stats.word_unigrams.get(&entry.target))
            })
            .copied()
            .unwrap_or(0);
        let frequency = *target_frequency
            .entry((entry.target.clone(), entry.frequency_lang.clone()))
            .or_insert(entry.frequency.max(corpus_frequency).max(1));
        let entry_id = u32::try_from(image_entries.len())
            .map_err(|_| "dictionary image entry table exceeded u32 ids".to_owned())?;

        exact_index.entry(normalized_key.clone()).or_default().push(entry_id);
        // Same construction as RankedLexiconEntry.alias_keys (roman_search_variants
        // minus the normalized key) so score_forms matches the heap path exactly.
        let mut this_entry_alias_ids = Vec::<u32>::new();
        for key in roman_search_variants(&entry.roman)
            .into_iter()
            .filter(|key| key != &normalized_key)
        {
            let alias_key_id = interner.intern(&key)?;
            this_entry_alias_ids.push(alias_key_id);
            push_dictionary_grams(&mut gram_index, &key, entry_id);
            alias_index.entry(key).or_default().push(entry_id);
        }
        push_dictionary_grams(&mut gram_index, &normalized_key, entry_id);

        let (first_tag_id, last_tag_id) = boundary_tag_ids(
            &entry.target,
            corpus_stats.map(|stats| &stats.dominant_word_tags),
            &mut interner,
        )?;
        image_entries.push(DictionaryImageEntryRecord {
            target_id,
            canonical_roman_id,
            normalized_key_id,
            frequency,
            frequency_lang_id,
            first_tag_id,
            last_tag_id,
        });
        entry_alias_ids.push(this_entry_alias_ids);
    }

    let legacy_indexes = build_legacy_dictionary_indexes(entries, corpus_stats);
    let entries_section = compile_dictionary_entry_section(&image_entries);
    let (entry_alias_refs, entry_alias_id_blob) = compile_dictionary_entry_alias_sections(&entry_alias_ids)?;
    let (exact_keys, exact_postings) = compile_dictionary_key_index(&mut interner, exact_index)?;
    let (alias_keys, alias_postings) = compile_dictionary_key_index(&mut interner, alias_index)?;
    let (gram_keys, gram_postings) = compile_dictionary_key_index(&mut interner, gram_index)?;
    let (legacy_roman_keys, legacy_roman_postings) =
        compile_dictionary_string_index(&mut interner, legacy_indexes.by_roman)?;
    let (legacy_normalized_keys, legacy_normalized_postings) =
        compile_dictionary_string_index(&mut interner, legacy_indexes.by_normalized)?;
    let (legacy_target_keys, legacy_target_postings) =
        compile_dictionary_string_index(&mut interner, legacy_indexes.by_target)?;
    let (legacy_prefix_keys, legacy_prefix_postings) =
        compile_dictionary_string_index(&mut interner, legacy_indexes.roman_prefix_index)?;
    let legacy_target_frequencies = compile_dictionary_frequency_index(&mut interner, legacy_indexes.target_frequency)?;

    // Key-index compilation may intern late keys; write string sections after
    // all sections have had a chance to assign string IDs.
    let (string_refs, string_data) = compile_dictionary_string_sections(&interner)?;

    write_dictionary_image_sections(&[
        (SECTION_STRING_REFS, string_refs),
        (SECTION_STRING_DATA, string_data),
        (SECTION_ENTRIES, entries_section),
        (SECTION_EXACT_KEYS, exact_keys),
        (SECTION_EXACT_POSTINGS, exact_postings),
        (SECTION_ALIAS_KEYS, alias_keys),
        (SECTION_ALIAS_POSTINGS, alias_postings),
        (SECTION_GRAM_KEYS, gram_keys),
        (SECTION_GRAM_POSTINGS, gram_postings),
        (SECTION_LEGACY_ROMAN_KEYS, legacy_roman_keys),
        (SECTION_LEGACY_ROMAN_POSTINGS, legacy_roman_postings),
        (SECTION_LEGACY_NORMALIZED_KEYS, legacy_normalized_keys),
        (SECTION_LEGACY_NORMALIZED_POSTINGS, legacy_normalized_postings),
        (SECTION_LEGACY_TARGET_KEYS, legacy_target_keys),
        (SECTION_LEGACY_TARGET_POSTINGS, legacy_target_postings),
        (SECTION_LEGACY_PREFIX_KEYS, legacy_prefix_keys),
        (SECTION_LEGACY_PREFIX_POSTINGS, legacy_prefix_postings),
        (SECTION_LEGACY_TARGET_FREQUENCIES, legacy_target_frequencies),
        (SECTION_ENTRY_ALIAS_REFS, entry_alias_refs),
        (SECTION_ENTRY_ALIAS_IDS, entry_alias_id_blob),
    ])
}

// The entry's first/last word boundary tag string ids, mirroring the runtime
// boundary_tags_for_target so the image carries the same POS tags the heap ranked
// table would compute. Returns MISSING_STRING_ID when a tag is absent.
fn boundary_tag_ids(
    target: &str,
    dominant_word_tags: Option<&HashMap<String, String>>,
    interner: &mut DictionaryImageInterner,
) -> Result<(u32, u32), String> {
    let Some(dominant) = dominant_word_tags else {
        return Ok((MISSING_STRING_ID, MISSING_STRING_ID));
    };
    let mut words = target.split_whitespace();
    let Some(first_word) = words.next() else {
        return Ok((MISSING_STRING_ID, MISSING_STRING_ID));
    };
    let last_word = words.last().unwrap_or(first_word);
    let intern_tag = |interner: &mut DictionaryImageInterner, word: &str| -> Result<u32, String> {
        match dominant.get(word) {
            Some(tag) => interner.intern(tag),
            None => Ok(MISSING_STRING_ID),
        }
    };
    let first_tag_id = intern_tag(interner, first_word)?;
    let last_tag_id = intern_tag(interner, last_word)?;
    Ok((first_tag_id, last_tag_id))
}

// Per-entry alias keys as a dense ragged array: REFS holds a (start, count) record
// per entry id (start is a u32 index into IDS); IDS is a flat u32 blob of string ids.
fn compile_dictionary_entry_alias_sections(entry_alias_ids: &[Vec<u32>]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut refs = Vec::with_capacity(entry_alias_ids.len() * ENTRY_ALIAS_REF_RECORD_LEN);
    let mut ids = Vec::new();
    for alias_ids in entry_alias_ids {
        let start = u32::try_from(ids.len() / 4)
            .map_err(|_| "dictionary image entry alias ids exceeded u32 offsets".to_owned())?;
        let count = u32::try_from(alias_ids.len())
            .map_err(|_| "dictionary image entry alias range exceeded u32 length".to_owned())?;
        write_u32(&mut refs, start);
        write_u32(&mut refs, count);
        for id in alias_ids {
            write_u32(&mut ids, *id);
        }
    }
    Ok((refs, ids))
}

struct LegacyDictionaryIndexes {
    by_roman: BTreeMap<String, Vec<String>>,
    by_normalized: BTreeMap<String, Vec<String>>,
    by_target: BTreeMap<String, Vec<String>>,
    target_frequency: BTreeMap<String, u32>,
    roman_prefix_index: BTreeMap<String, Vec<String>>,
}

fn build_legacy_dictionary_indexes(
    entries: &[BuildLexiconEntry],
    corpus_stats: Option<&BuildCorpusFrequencyStats>,
) -> LegacyDictionaryIndexes {
    let target_frequency = target_frequency_map_for_dictionary_image(entries, corpus_stats);
    let mut by_roman = BTreeMap::<String, Vec<String>>::new();
    let mut by_normalized = BTreeMap::<String, Vec<String>>::new();
    let mut by_target = BTreeMap::<String, Vec<String>>::new();

    for entry in entries {
        by_roman
            .entry(entry.roman.clone())
            .or_default()
            .push(entry.target.clone());
        by_normalized
            .entry(normalize(&entry.roman))
            .or_default()
            .push(entry.target.clone());
        by_target
            .entry(entry.target.clone())
            .or_default()
            .push(normalize(&entry.roman));
    }

    for values in by_roman.values_mut() {
        sort_targets_by_dictionary_frequency(values, &target_frequency);
    }
    for values in by_normalized.values_mut() {
        sort_targets_by_dictionary_frequency(values, &target_frequency);
    }

    let mut sorted_romans = by_roman.keys().cloned().collect::<Vec<_>>();
    sorted_romans.sort_by(|left, right| left.len().cmp(&right.len()).then_with(|| left.cmp(right)));
    let mut roman_prefix_index = BTreeMap::<String, Vec<String>>::new();
    for roman in sorted_romans {
        let normalized = normalize(&roman);
        for prefix_len in 1..=3 {
            let prefix = normalized.chars().take(prefix_len).collect::<String>();
            if prefix.chars().count() != prefix_len {
                break;
            }
            roman_prefix_index.entry(prefix).or_default().push(roman.clone());
        }
    }

    LegacyDictionaryIndexes {
        by_roman,
        by_normalized,
        by_target,
        target_frequency,
        roman_prefix_index,
    }
}

fn target_frequency_map_for_dictionary_image(
    entries: &[BuildLexiconEntry],
    corpus_stats: Option<&BuildCorpusFrequencyStats>,
) -> BTreeMap<String, u32> {
    let mut frequency = BTreeMap::<String, u32>::new();
    for entry in entries {
        let corpus_frequency = corpus_stats
            .map(|stats| {
                stats
                    .surface_unigrams
                    .get(&entry.target)
                    .or_else(|| stats.word_unigrams.get(&entry.target))
                    .copied()
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        let effective = entry.frequency.max(corpus_frequency).max(1);
        frequency
            .entry(entry.target.clone())
            .and_modify(|current| *current = (*current).max(effective))
            .or_insert(effective);
    }
    frequency
}

fn sort_targets_by_dictionary_frequency(values: &mut [String], target_frequency: &BTreeMap<String, u32>) {
    values.sort_by(|left, right| {
        target_frequency
            .get(right)
            .copied()
            .unwrap_or(1)
            .cmp(&target_frequency.get(left).copied().unwrap_or(1))
            .then_with(|| left.cmp(right))
    });
}

fn push_dictionary_grams(index: &mut BTreeMap<String, Vec<u32>>, input: &str, entry_id: u32) {
    let chars = input.chars().collect::<Vec<_>>();
    if chars.len() < 2 {
        return;
    }
    for start in 0..=chars.len() - 2 {
        index
            .entry(chars[start..start + 2].iter().collect())
            .or_default()
            .push(entry_id);
    }
}

fn compile_dictionary_string_sections(interner: &DictionaryImageInterner) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut refs = Vec::with_capacity(interner.strings.len() * STRING_REF_RECORD_LEN);
    let mut data = Vec::new();
    for value in &interner.strings {
        let offset =
            u32::try_from(data.len()).map_err(|_| "dictionary image string blob exceeded u32 offsets".to_owned())?;
        let len = u32::try_from(value.len()).map_err(|_| "dictionary image string length exceeded u32".to_owned())?;
        write_u32(&mut refs, offset);
        write_u32(&mut refs, len);
        data.extend_from_slice(value.as_bytes());
    }
    Ok((refs, data))
}

fn compile_dictionary_entry_section(entries: &[DictionaryImageEntryRecord]) -> Vec<u8> {
    let mut output = Vec::with_capacity(entries.len() * ENTRY_RECORD_LEN);
    for entry in entries {
        write_u32(&mut output, entry.target_id);
        write_u32(&mut output, entry.canonical_roman_id);
        write_u32(&mut output, entry.normalized_key_id);
        write_u32(&mut output, entry.frequency);
        write_u32(&mut output, entry.frequency_lang_id);
        write_u32(&mut output, entry.first_tag_id);
        write_u32(&mut output, entry.last_tag_id);
    }
    output
}

fn compile_dictionary_key_index(
    interner: &mut DictionaryImageInterner,
    index: BTreeMap<String, Vec<u32>>,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut keys = Vec::with_capacity(index.len() * KEY_RANGE_RECORD_LEN);
    let mut postings = Vec::new();
    for (key, ids) in index {
        let key_id = interner.intern(&key)?;
        let start = u32::try_from(postings.len() / 4)
            .map_err(|_| "dictionary image postings exceeded u32 offsets".to_owned())?;
        let len =
            u32::try_from(ids.len()).map_err(|_| "dictionary image posting range exceeded u32 length".to_owned())?;
        write_u32(&mut keys, key_id);
        write_u32(&mut keys, start);
        write_u32(&mut keys, len);
        for id in ids {
            write_u32(&mut postings, id);
        }
    }
    Ok((keys, postings))
}

fn compile_dictionary_string_index(
    interner: &mut DictionaryImageInterner,
    index: BTreeMap<String, Vec<String>>,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut keys = Vec::with_capacity(index.len() * KEY_RANGE_RECORD_LEN);
    let mut postings = Vec::new();
    for (key, values) in index {
        let key_id = interner.intern(&key)?;
        let start = u32::try_from(postings.len() / 4)
            .map_err(|_| "dictionary image string postings exceeded u32 offsets".to_owned())?;
        let len = u32::try_from(values.len())
            .map_err(|_| "dictionary image string posting range exceeded u32 length".to_owned())?;
        write_u32(&mut keys, key_id);
        write_u32(&mut keys, start);
        write_u32(&mut keys, len);
        for value in values {
            let value_id = interner.intern(&value)?;
            write_u32(&mut postings, value_id);
        }
    }
    Ok((keys, postings))
}

fn compile_dictionary_frequency_index(
    interner: &mut DictionaryImageInterner,
    index: BTreeMap<String, u32>,
) -> Result<Vec<u8>, String> {
    let mut records = Vec::with_capacity(index.len() * STRING_U32_RECORD_LEN);
    for (key, frequency) in index {
        let key_id = interner.intern(&key)?;
        write_u32(&mut records, key_id);
        write_u32(&mut records, frequency);
    }
    Ok(records)
}

fn write_dictionary_image_sections(sections: &[(u32, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let section_count =
        u32::try_from(sections.len()).map_err(|_| "dictionary image section count exceeded u32".to_owned())?;
    if section_count != DICTIONARY_IMAGE_SECTION_COUNT {
        return Err("dictionary image section count does not match format".to_owned());
    }

    let table_len = sections
        .len()
        .checked_mul(SECTION_RECORD_LEN)
        .and_then(|len| HEADER_LEN.checked_add(len))
        .ok_or_else(|| "dictionary image header exceeded usize".to_owned())?;
    let mut output = Vec::with_capacity(table_len + sections.iter().map(|(_, data)| data.len()).sum::<usize>());
    output.extend_from_slice(DICTIONARY_IMAGE_MAGIC);
    write_u32(&mut output, DICTIONARY_IMAGE_SCHEMA_VERSION);
    write_u32(&mut output, section_count);

    let mut section_offset =
        u32::try_from(table_len).map_err(|_| "dictionary image section offset exceeded u32".to_owned())?;
    for (id, data) in sections {
        let len = u32::try_from(data.len()).map_err(|_| "dictionary image section length exceeded u32".to_owned())?;
        write_u32(&mut output, *id);
        write_u32(&mut output, section_offset);
        write_u32(&mut output, len);
        section_offset = section_offset
            .checked_add(len)
            .ok_or_else(|| "dictionary image section offsets exceeded u32".to_owned())?;
    }

    for (_, data) in sections {
        output.extend_from_slice(data);
    }
    Ok(output)
}

fn parse_additional_csv_entries(source: &str) -> Vec<BuildLexiconEntry> {
    let mut entries = Vec::new();
    let mut first_row = true;

    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let mut fields = match parse_csv_fields(line, line_no + 1) {
            Ok(fields) => fields,
            Err(_) => continue,
        };
        if fields.len() != 2 {
            continue;
        }
        if line_no == 0 {
            fields[0] = fields[0].trim_start_matches('\u{feff}').to_owned();
        }

        let roman = fields.remove(0).trim().to_owned();
        let target = fields.remove(0).trim().to_owned();

        if first_row {
            let left = roman.to_ascii_lowercase();
            let right = target.to_ascii_lowercase();
            if (left == "roman" && right == "target") || (left == "english" && right == "khmer") {
                first_row = false;
                continue;
            }
        }
        first_row = false;

        if roman.is_empty() || target.is_empty() {
            continue;
        }
        entries.push(BuildLexiconEntry::default_frequency(roman, target));
    }

    entries
}

fn parse_lexicon_entries(source: &str, source_format: LexiconSourceFormat) -> Result<Vec<BuildLexiconEntry>, String> {
    match source_format {
        LexiconSourceFormat::Csv => parse_csv_entries(source),
        LexiconSourceFormat::Tsv => parse_tsv_entries(source),
    }
}

fn parse_tsv_entries(source: &str) -> Result<Vec<BuildLexiconEntry>, String> {
    let mut entries = Vec::new();
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let Some((roman, target)) = line.split_once('\t') else {
            return Err(format!("invalid TSV data format on line {}", line_no + 1));
        };
        entries.push(BuildLexiconEntry::default_frequency(
            roman.to_owned(),
            target.to_owned(),
        ));
    }
    Ok(entries)
}

fn parse_csv_entries(source: &str) -> Result<Vec<BuildLexiconEntry>, String> {
    let mut entries = Vec::new();
    let mut first_row = true;
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = parse_csv_fields(line, line_no + 1)?;
        if !matches!(fields.len(), 2 | 3 | 4) {
            return Err(format!(
                "invalid CSV data format on line {}: expected 2, 3, or 4 columns, got {}",
                line_no + 1,
                fields.len()
            ));
        }
        if line_no == 0 {
            fields[0] = fields[0].trim_start_matches('\u{feff}').to_owned();
        }
        if first_row
            && fields[0].trim().eq_ignore_ascii_case("roman")
            && fields[1].trim().eq_ignore_ascii_case("target")
        {
            first_row = false;
            continue;
        }
        first_row = false;
        let frequency = parse_lexicon_frequency(fields.get(2).map(String::as_str).unwrap_or(""), line_no + 1, true)?;
        let frequency_lang = if let Some(value) = fields.get(3) {
            let value = value.trim();
            if value.is_empty() {
                return Err(format!(
                    "invalid CSV data format on line {}: freq_lang is required",
                    line_no + 1
                ));
            }
            validate_frequency_lang(value, line_no + 1)?;
            value.to_owned()
        } else {
            "km".to_owned()
        };
        entries.push(BuildLexiconEntry::new(
            fields.remove(0),
            fields.remove(0),
            frequency,
            frequency_lang,
        ));
    }
    Ok(entries)
}

fn parse_lexicon_frequency(raw: &str, line_no: usize, allow_blank: bool) -> Result<u32, String> {
    let value = raw.trim();
    if value.is_empty() {
        if allow_blank {
            return Ok(1);
        }
        return Err(format!(
            "invalid CSV data format on line {line_no}: frequency is required"
        ));
    }
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("invalid CSV data format on line {line_no}: frequency must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!(
            "invalid CSV data format on line {line_no}: frequency must be a positive integer"
        ));
    }
    Ok(parsed)
}

fn validate_frequency_lang(value: &str, line_no: usize) -> Result<(), String> {
    if matches!(value, "km" | "en" | "ja" | "zh" | "ko") {
        Ok(())
    } else {
        Err(format!(
            "invalid CSV data format on line {line_no}: unsupported freq_lang '{value}'"
        ))
    }
}

fn parse_csv_fields(line: &str, line_no: usize) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '"' => {
                if current.is_empty() {
                    in_quotes = true;
                } else {
                    return Err(format!("invalid CSV data format on line {}: unexpected quote", line_no));
                }
            }
            ',' => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }

    if in_quotes {
        return Err(format!(
            "invalid CSV data format on line {}: unterminated quote",
            line_no
        ));
    }

    fields.push(current);
    Ok(fields)
}

fn compile_khpos_stats(
    train_source: &str,
    tag_source: &str,
    options: BuildDataBuildOptions,
) -> Result<CompiledKhposStats, String> {
    let train_lines = train_source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let tag_lines = tag_source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if train_lines.len() != tag_lines.len() {
        return Err(format!(
            "khPOS sentence count mismatch: {} train lines vs {} tag lines",
            train_lines.len(),
            tag_lines.len()
        ));
    }

    let mut word_unigrams = HashMap::<String, u32>::new();
    let mut word_bigrams = HashMap::<(String, String), u32>::new();
    let mut surface_unigrams = HashMap::<String, u32>::new();
    let mut tag_unigrams = HashMap::<String, u32>::new();
    let mut tag_bigrams = HashMap::<(String, String), u32>::new();
    let mut word_tag_counts = HashMap::<String, HashMap<String, u32>>::new();

    for (line_no, (train_line, tag_line)) in train_lines.iter().zip(tag_lines.iter()).enumerate() {
        let tagged_tokens = train_line.split_whitespace().collect::<Vec<_>>();
        let tags = tag_line.split_whitespace().collect::<Vec<_>>();
        if tagged_tokens.len() != tags.len() {
            return Err(format!(
                "khPOS token/tag mismatch on line {}: {} tagged tokens vs {} tags",
                line_no + 1,
                tagged_tokens.len(),
                tags.len()
            ));
        }

        let mut words = Vec::<String>::with_capacity(tagged_tokens.len());
        for (column, (tagged, expected_tag)) in tagged_tokens.iter().zip(tags.iter()).enumerate() {
            let Some((word, observed_tag)) = tagged.rsplit_once('/') else {
                return Err(format!(
                    "khPOS token missing word/tag separator on line {}, column {}",
                    line_no + 1,
                    column + 1
                ));
            };
            if word.is_empty() || observed_tag.is_empty() {
                return Err(format!(
                    "khPOS token has empty word/tag part on line {}, column {}",
                    line_no + 1,
                    column + 1
                ));
            }
            if observed_tag != *expected_tag {
                return Err(format!(
                    "khPOS tag mismatch on line {}, column {}: train token has '{}' but tag file has '{}'",
                    line_no + 1,
                    column + 1,
                    observed_tag,
                    expected_tag
                ));
            }

            let word = word.to_owned();
            let tag = (*expected_tag).to_owned();
            *word_unigrams.entry(word.clone()).or_default() += 1;
            *tag_unigrams.entry(tag.clone()).or_default() += 1;
            *word_tag_counts.entry(word.clone()).or_default().entry(tag).or_default() += 1;
            words.push(word);
        }

        for pair in words.windows(2) {
            *word_bigrams.entry((pair[0].clone(), pair[1].clone())).or_default() += 1;
        }
        for start in 0..words.len() {
            let mut joined = String::new();
            for token in words.iter().skip(start).take(MAX_JOINED_SURFACE_TOKENS) {
                joined.push_str(token);
                *surface_unigrams.entry(joined.clone()).or_default() += 1;
            }
        }
        for pair in tags.windows(2) {
            *tag_bigrams.entry((pair[0].to_owned(), pair[1].to_owned())).or_default() += 1;
        }
    }

    if options.khpos_surface_min_count > 1 {
        surface_unigrams.retain(|_, count| *count >= options.khpos_surface_min_count);
    }

    if let Some(limit) = options.khpos_surface_top_n {
        trim_map_to_top_n(&mut surface_unigrams, limit);
    }

    let mut dominant_tags = word_tag_counts
        .into_iter()
        .map(|(word, tags)| {
            let mut tags = tags.into_iter().collect::<Vec<_>>();
            tags.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            let (tag, count) = tags.into_iter().next().expect("dominant tag source must be non-empty");
            (word, tag, count)
        })
        .collect::<Vec<_>>();
    dominant_tags.sort_by(|left, right| left.0.cmp(&right.0));

    let mut output = Vec::new();
    output.extend_from_slice(KHPOS_MAGIC);
    write_string_count_map(&mut output, &word_unigrams)?;
    write_pair_count_map(&mut output, &word_bigrams)?;
    write_string_count_map(&mut output, &surface_unigrams)?;
    write_string_count_map(&mut output, &tag_unigrams)?;
    write_pair_count_map(&mut output, &tag_bigrams)?;
    write_dominant_tags(&mut output, &dominant_tags)?;
    let dominant_word_tags = dominant_tags
        .iter()
        .map(|(word, tag, _)| (word.clone(), tag.clone()))
        .collect::<HashMap<_, _>>();
    Ok(CompiledKhposStats {
        bytes: output,
        frequency_stats: BuildCorpusFrequencyStats {
            word_unigrams,
            surface_unigrams,
            dominant_word_tags,
        },
    })
}

fn trim_map_to_top_n(map: &mut HashMap<String, u32>, limit: usize) {
    if map.len() <= limit {
        return;
    }
    let mut ranked = map
        .iter()
        .map(|(token, count)| (token.clone(), *count))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let keep = ranked
        .into_iter()
        .take(limit)
        .map(|(token, _)| token)
        .collect::<std::collections::HashSet<_>>();
    map.retain(|token, _| keep.contains(token));
}

fn write_string_count_map(output: &mut Vec<u8>, map: &HashMap<String, u32>) -> Result<(), String> {
    let mut records = map.iter().collect::<Vec<_>>();
    records.sort_by(|left, right| left.0.cmp(right.0));
    write_u32(output, records.len() as u32);
    for (text, count) in records {
        write_string(output, text)?;
        write_u32(output, *count);
    }
    Ok(())
}

fn write_pair_count_map(output: &mut Vec<u8>, map: &HashMap<(String, String), u32>) -> Result<(), String> {
    let mut records = map.iter().collect::<Vec<_>>();
    records.sort_by(|left, right| left.0.cmp(right.0));
    write_u32(output, records.len() as u32);
    for ((left, right), count) in records {
        write_string(output, left)?;
        write_string(output, right)?;
        write_u32(output, *count);
    }
    Ok(())
}

fn write_dominant_tags(output: &mut Vec<u8>, records: &[(String, String, u32)]) -> Result<(), String> {
    write_u32(output, records.len() as u32);
    for (word, tag, count) in records {
        write_string(output, word)?;
        write_string(output, tag)?;
        write_u32(output, *count);
    }
    Ok(())
}

fn write_string(output: &mut Vec<u8>, text: &str) -> Result<(), String> {
    if text.contains('\0') {
        return Err(format!("khPOS compiled data does not support NUL bytes in '{}'", text));
    }
    output.extend_from_slice(text.as_bytes());
    output.push(0);
    Ok(())
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}
