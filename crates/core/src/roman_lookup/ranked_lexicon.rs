use super::*;

impl RankedLexiconEntry {
    pub(crate) fn score_forms(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.normalized_key.as_str())
            .chain(self.alias_keys.iter().map(String::as_str))
            .filter(|key| !key.starts_with("sk:"))
    }
}

impl RankedLexicon {
    pub(crate) fn from_entries(entries: &[Entry], corpus_stats: &CorpusStats) -> Self {
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
