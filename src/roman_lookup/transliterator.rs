use super::*;

#[cfg(target_arch = "wasm32")]
fn startup_trace_now_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or(0.0)
}

#[cfg(not(target_arch = "wasm32"))]
fn startup_trace_now_ms() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;

    static STARTED_AT: OnceLock<Instant> = OnceLock::new();
    let started_at = STARTED_AT.get_or_init(Instant::now);
    started_at.elapsed().as_secs_f64() * 1000.0
}

fn startup_trace_log(stage: &str) {
    let message = format!("[startup] {stage} t_ms={:.2}", startup_trace_now_ms());
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&message.clone().into());
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{message}");
}

impl Transliterator {
    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_default_data() -> Result<Self> {
        Self::from_default_data_with_config(DecoderConfig::default())
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_tsv_path(path: impl AsRef<Path>) -> Result<Self> {
        let source = fs::read_to_string(path)?;
        Self::from_tsv_str_with_config(&source, DecoderConfig::default())
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_tsv_str(source: &str) -> Result<Self> {
        Self::from_tsv_str_with_config(source, DecoderConfig::default())
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_csv_path(path: impl AsRef<Path>) -> Result<Self> {
        let source = fs::read_to_string(path)?;
        Self::from_csv_str_with_config(&source, DecoderConfig::default())
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_csv_str(source: &str) -> Result<Self> {
        Self::from_csv_str_with_config(source, DecoderConfig::default())
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_data_path(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_data_path_with_config(path, DecoderConfig::default())
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_default_data_with_config(config: DecoderConfig) -> Result<Self> {
        let entries = parse_compiled_lexicon(DEFAULT_COMPILED_DATA)?;
        let corpus_stats = parse_compiled_khpos_stats(DEFAULT_COMPILED_KHPOS_STATS)?;
        let next_word = parse_compiled_next_word_stats(DEFAULT_COMPILED_NEXT_WORD_STATS)?;
        Self::from_entries_with_config(entries, corpus_stats, next_word, config)
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_tsv_path_with_config(path: impl AsRef<Path>, config: DecoderConfig) -> Result<Self> {
        let source = fs::read_to_string(path)?;
        Self::from_tsv_str_with_config(&source, config)
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_tsv_str_with_config(source: &str, config: DecoderConfig) -> Result<Self> {
        let entries = parse_tsv(source)?;
        let corpus_stats = CorpusStats::from_default_data()?;
        let next_word = NextWordStats::from_default_data()?;
        Self::from_entries_with_config(entries, corpus_stats, next_word, config)
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_csv_path_with_config(path: impl AsRef<Path>, config: DecoderConfig) -> Result<Self> {
        let source = fs::read_to_string(path)?;
        Self::from_csv_str_with_config(&source, config)
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_csv_str_with_config(source: &str, config: DecoderConfig) -> Result<Self> {
        let entries = parse_csv(source)?;
        let corpus_stats = CorpusStats::from_default_data()?;
        let next_word = NextWordStats::from_default_data()?;
        Self::from_entries_with_config(entries, corpus_stats, next_word, config)
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub fn from_data_path_with_config(path: impl AsRef<Path>, config: DecoderConfig) -> Result<Self> {
        let path_ref = path.as_ref();
        let source = fs::read_to_string(path_ref)?;
        let entries = if path_ref
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("csv"))
            .unwrap_or(false)
        {
            parse_csv(&source)?
        } else {
            parse_tsv(&source)?
        };
        let corpus_stats = CorpusStats::from_default_data()?;
        let next_word = NextWordStats::from_default_data()?;
        Self::from_entries_with_config(entries, corpus_stats, next_word, config)
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    fn from_entries_with_config(
        entries: Vec<Entry>,
        corpus_stats: CorpusStats,
        next_word: NextWordStats,
        config: DecoderConfig,
    ) -> Result<Self> {
        let legacy = Arc::new(LegacyData::from_entries_with_stats(entries, corpus_stats, next_word));
        let composer = ComposerTable::from_entries(legacy.entries());
        let decoder = DecoderManager::new(
            composer,
            LegacyDecoder::new(Arc::clone(&legacy)),
            (config.mode != DecoderMode::Legacy).then(|| WfstDecoder::new(Arc::clone(&legacy), config.clone())),
            config,
        );
        Ok(Self { legacy, decoder })
    }

    /// Build a `Transliterator` from externally-provided compiled binary blobs.
    /// On `wasm32` with the `fetch-data` feature these blobs are fetched at runtime
    /// instead of being baked into the binary via `include_bytes!`.
    pub fn from_compiled_bytes(lexicon: &[u8], khpos: &[u8], next_word: &[u8], config: DecoderConfig) -> Result<Self> {
        startup_trace_log("Transliterator::from_compiled_bytes.start");
        let entries = parse_compiled_lexicon(lexicon)?;
        let corpus_stats = parse_compiled_khpos_stats(khpos)?;
        let next_word_stats = parse_compiled_next_word_stats(next_word)?;
        let legacy = Arc::new(LegacyData::from_entries_with_stats(
            entries,
            corpus_stats,
            next_word_stats,
        ));
        let composer = ComposerTable::from_entries(legacy.entries());
        let decoder = DecoderManager::new(
            composer,
            LegacyDecoder::new(Arc::clone(&legacy)),
            (config.mode != DecoderMode::Legacy).then(|| WfstDecoder::new(Arc::clone(&legacy), config.clone())),
            config,
        );
        startup_trace_log("Transliterator::from_compiled_bytes.end");
        Ok(Self { legacy, decoder })
    }

    /// Build a minimal phase-A transliterator from compiled lexicon bytes.
    /// This path intentionally skips heavy fuzzy/ranking/composer structures so web startup can
    /// unlock basic suggestions faster on constrained devices (notably iOS Safari).
    pub fn from_phase_a_bytes(lexicon: &[u8], config: DecoderConfig) -> Result<Self> {
        startup_trace_log("Transliterator::from_phase_a_bytes.start");
        let entries = parse_compiled_lexicon(lexicon)?;
        let legacy = Arc::new(LegacyData::from_entries_phase_a(entries));
        // Phase A keeps legacy lexical suggestions available but defers expensive
        // composer trie construction to full engine promotion.
        let composer = ComposerTable::empty();
        let decoder = DecoderManager::new(
            composer,
            LegacyDecoder::new(Arc::clone(&legacy)),
            (config.mode != DecoderMode::Legacy).then(|| WfstDecoder::new(Arc::clone(&legacy), config.clone())),
            config,
        );
        startup_trace_log("Transliterator::from_phase_a_bytes.end");
        Ok(Self { legacy, decoder })
    }

    /// Build a `Transliterator` from compiled lexicon bytes without khPOS corpus stats.
    /// This is used for fast phase-A startup where legacy suggestions are enough to unlock typing.
    pub fn from_compiled_lexicon_bytes(lexicon: &[u8], config: DecoderConfig) -> Result<Self> {
        Self::from_phase_a_bytes(lexicon, config)
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

    pub fn next_word_suggestions(
        &self,
        previous_token: &str,
        sentence_start: bool,
        history: &HashMap<String, usize>,
    ) -> Vec<String> {
        self.legacy
            .next_word_suggestions(previous_token, sentence_start, history)
    }

    pub fn infer_next_word_context_suffix(&self, text_before_caret: &str) -> Option<String> {
        self.legacy.infer_next_word_context_suffix(text_before_caret)
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

    pub fn exact_match_roman_variants(&self, input: &str, target: &str) -> Vec<String> {
        self.legacy.exact_match_roman_variants(input, target)
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

fn is_roman_letter(ch: char) -> bool {
    ch.is_ascii() && (ch.is_ascii_alphabetic() || ch == '_')
}

fn is_keycap_token_char(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, '!' | '"' | '#' | '$' | '%' | '&' | '\'' | '(' | ')' | '~' | '=')
}

fn is_period(ch: char) -> bool {
    ch == '.'
}
