use super::*;

impl RankedLexiconEntry {
    pub(crate) fn score_forms(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.normalized_key.as_str())
            .chain(self.alias_keys.iter().map(String::as_str))
            .filter(|key| !key.starts_with("sk:"))
    }
}

impl RankedLexicon {
    #[allow(dead_code)]
    pub(crate) fn from_entries(entries: &[Entry], corpus_stats: CorpusStats) -> Self {
        Self::from_entries_with_stage_logger(entries, corpus_stats, |_, _| {})
    }

    pub(crate) fn from_entries_with_stage_logger(
        entries: &[Entry],
        corpus_stats: CorpusStats,
        log_stage: impl FnMut(&str, f64),
    ) -> Self {
        Self::from_entries_with_stage_logger_and_index_mode(
            entries,
            corpus_stats,
            RankedLookupIndexMode::BuildRetrievalIndexes,
            log_stage,
        )
    }

    pub(crate) fn from_entries_with_stage_logger_and_index_mode(
        entries: &[Entry],
        corpus_stats: CorpusStats,
        index_mode: RankedLookupIndexMode,
        mut log_stage: impl FnMut(&str, f64),
    ) -> Self {
        let build_retrieval_indexes = index_mode == RankedLookupIndexMode::BuildRetrievalIndexes;
        let mut ranked = Self::default();
        ranked.entries = Vec::with_capacity(entries.len());
        if build_retrieval_indexes {
            ranked.exact_index = HashMap::with_capacity(entries.len());
            ranked.alias_index = HashMap::with_capacity(entries.len().saturating_mul(2));
            ranked.gram_index = HashMap::with_capacity(entries.len().saturating_mul(4));
        }
        ranked.word_unigrams = HashMap::with_capacity(entries.len());
        ranked.word_bigrams = HashMap::with_capacity(entries.len() / 4);
        let mut target_frequency = HashMap::<(String, String), u32>::new();
        let mut entry_frequencies = Vec::<u32>::with_capacity(entries.len());
        let started = start_stage_timer();
        for entry in entries {
            let corpus_frequency = corpus_stats
                .surface_unigrams
                .get(&entry.target)
                .or_else(|| corpus_stats.word_unigrams.get(&entry.target))
                .copied()
                .unwrap_or(0);
            let effective_frequency = entry.frequency.max(corpus_frequency).max(1);
            let frequency = *target_frequency
                .entry((entry.target.clone(), entry.frequency_lang.clone()))
                .or_insert(effective_frequency);
            entry_frequencies.push(frequency);
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
        log_stage("entry_frequency", elapsed_stage_ms(started));

        let started = start_stage_timer();
        for (entry, frequency) in entries.iter().zip(entry_frequencies) {
            let normalized_key = normalize(&entry.roman);
            if normalized_key.is_empty() {
                continue;
            }
            let alias_keys = roman_search_variants(&entry.roman)
                .into_iter()
                .filter(|key| key != &normalized_key)
                .collect::<Vec<_>>();
            let (first_tag, last_tag) = boundary_tags_for_target(&entry.target, &corpus_stats);
            let ranked_entry = RankedLexiconEntry {
                target: entry.target.clone(),
                canonical_roman: entry.roman.clone(),
                normalized_key: normalized_key.clone(),
                alias_keys,
                frequency,
                frequency_lang: entry.frequency_lang.clone(),
                first_tag,
                last_tag,
            };
            let entry_index = ranked.entries.len();
            if build_retrieval_indexes {
                ranked
                    .exact_index
                    .entry(normalized_key.clone())
                    .or_default()
                    .push(entry_index);
            }

            for key in &ranked_entry.alias_keys {
                if build_retrieval_indexes {
                    push_grams(&mut ranked.gram_index, &key, entry_index);
                    ranked.alias_index.entry(key.clone()).or_default().push(entry_index);
                }
            }
            if build_retrieval_indexes {
                push_grams(&mut ranked.gram_index, &normalized_key, entry_index);
            }
            ranked.entries.push(ranked_entry);
        }
        log_stage("entry_indexes", elapsed_stage_ms(started));

        let started = start_stage_timer();
        ranked.corpus_word_unigrams = corpus_stats.word_unigrams;
        ranked.corpus_word_bigrams = corpus_stats.word_bigrams;
        ranked.corpus_surface_unigrams = corpus_stats.surface_unigrams;
        ranked.tag_unigrams = corpus_stats.tag_unigrams;
        ranked.tag_bigrams = corpus_stats.tag_bigrams;
        log_stage("move_corpus_stats", elapsed_stage_ms(started));

        ranked
    }
}

fn push_grams(index: &mut HashMap<String, Vec<usize>>, input: &str, entry_index: usize) {
    let chars = input.chars().collect::<Vec<_>>();
    if chars.len() < 2 {
        return;
    }
    for start in 0..=chars.len() - 2 {
        index
            .entry(chars[start..start + 2].iter().collect())
            .or_default()
            .push(entry_index);
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
