use super::dictionary_image::DictionaryEntryView;
use super::search_index::SearchIndex;
use super::*;

// A ranked entry whose fields come from either the heap ranked table or the
// zero-copy dictionary image, so the decoder can read entries without retaining
// the heap Vec<RankedLexiconEntry> when the image is present.
pub(crate) enum RankedEntryView<'a> {
    Heap(&'a RankedLexiconEntry),
    Image {
        image: DictionaryImageView<'a>,
        entry: DictionaryEntryView<'a>,
        entry_id: u32,
    },
}

impl<'a> RankedEntryView<'a> {
    pub(crate) fn frequency(&self) -> u32 {
        match self {
            Self::Heap(entry) => entry.frequency,
            Self::Image { entry, .. } => entry.frequency().expect("dictionary image entry frequency"),
        }
    }

    pub(crate) fn normalized_key(&self) -> &'a str {
        match self {
            Self::Heap(entry) => entry.normalized_key.as_str(),
            Self::Image { entry, .. } => entry.normalized_key().expect("dictionary image entry normalized_key"),
        }
    }

    pub(crate) fn target(&self) -> &'a str {
        match self {
            Self::Heap(entry) => entry.target.as_str(),
            Self::Image { entry, .. } => entry.target().expect("dictionary image entry target"),
        }
    }

    pub(crate) fn canonical_roman(&self) -> &'a str {
        match self {
            Self::Heap(entry) => entry.canonical_roman.as_str(),
            Self::Image { entry, .. } => entry.canonical_roman().expect("dictionary image entry canonical_roman"),
        }
    }

    pub(crate) fn frequency_lang(&self) -> &'a str {
        match self {
            Self::Heap(entry) => entry.frequency_lang.as_str(),
            Self::Image { entry, .. } => entry.frequency_lang().expect("dictionary image entry frequency_lang"),
        }
    }

    pub(crate) fn first_tag(&self) -> Option<&'a str> {
        match self {
            Self::Heap(entry) => entry.first_tag.as_deref(),
            Self::Image { entry, .. } => entry.first_tag().expect("dictionary image entry first_tag"),
        }
    }

    pub(crate) fn last_tag(&self) -> Option<&'a str> {
        match self {
            Self::Heap(entry) => entry.last_tag.as_deref(),
            Self::Image { entry, .. } => entry.last_tag().expect("dictionary image entry last_tag"),
        }
    }

    // normalized_key + alias_keys, excluding "sk:"-prefixed keys (matches
    // RankedLexiconEntry::score_forms).
    pub(crate) fn score_forms(&self) -> Vec<&'a str> {
        match self {
            Self::Heap(entry) => entry.score_forms().collect(),
            Self::Image { image, entry_id, .. } => {
                let normalized = self.normalized_key();
                let alias_keys = image
                    .entry_alias_keys(*entry_id)
                    .expect("dictionary image entry alias_keys");
                std::iter::once(normalized)
                    .chain(alias_keys)
                    .filter(|key| !key.starts_with("sk:"))
                    .collect()
            }
        }
    }
}

impl LegacyData {
    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    #[allow(dead_code)]
    pub(crate) fn from_entries(entries: Vec<Entry>) -> Self {
        let corpus_stats = CorpusStats::from_default_data().expect("embedded khPOS stats must load");
        let next_word = NextWordStats::from_default_data().expect("embedded next-word stats must load");
        Self::from_entries_with_stats(entries, corpus_stats, next_word)
    }

    pub(crate) fn from_entries_phase_a(entries: Vec<Entry>) -> Self {
        let maps = Self::build_lookup_maps(&entries);
        Self {
            entries,
            by_roman: maps.by_roman,
            by_normalized: maps.by_normalized,
            by_target: maps.by_target,
            target_frequency: maps.target_frequency,
            roman_normalized: maps.roman_normalized,
            roman_prefix_index: maps.roman_prefix_index,
            // Phase A avoids building the heavyweight fuzzy gram index.
            index: SearchIndex::new(&[], true, 2, 3),
            // Phase A avoids khPOS-derived ranking structures.
            ranked: RankedLexicon::default(),
            dictionary_image: None,
            // Phase A defers next-word n-gram stats until full engine promotion.
            next_word: NextWordStats::default(),
            next_word_max_context_chars: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_ranked_for_tests(ranked: RankedLexicon) -> Self {
        Self {
            entries: Vec::new(),
            by_roman: HashMap::new(),
            by_normalized: HashMap::new(),
            by_target: HashMap::new(),
            target_frequency: HashMap::new(),
            roman_normalized: HashMap::new(),
            roman_prefix_index: HashMap::new(),
            index: SearchIndex::new(&[], true, 2, 3),
            ranked,
            dictionary_image: None,
            next_word: NextWordStats::default(),
            next_word_max_context_chars: 0,
        }
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub(crate) fn from_entries_phase_a_with_dictionary_image(
        entries: Vec<Entry>,
        dictionary_image: DictionaryImageView<'static>,
    ) -> Self {
        Self {
            entries,
            by_roman: HashMap::new(),
            by_normalized: HashMap::new(),
            by_target: HashMap::new(),
            target_frequency: HashMap::new(),
            roman_normalized: HashMap::new(),
            roman_prefix_index: HashMap::new(),
            // Phase A avoids building the heavyweight fuzzy gram index.
            index: SearchIndex::new(&[], true, 2, 3),
            // Phase A avoids khPOS-derived ranking structures.
            ranked: RankedLexicon::default(),
            dictionary_image: Some(dictionary_image),
            // Phase A defers next-word n-gram stats until full engine promotion.
            next_word: NextWordStats::default(),
            next_word_max_context_chars: 0,
        }
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub(crate) fn from_entries_with_stats(
        entries: Vec<Entry>,
        corpus_stats: CorpusStats,
        next_word: NextWordStats,
    ) -> Self {
        Self::from_entries_with_stats_and_stage_logger(entries, corpus_stats, next_word, |_, _| {})
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub(crate) fn from_entries_with_stats_and_stage_logger(
        entries: Vec<Entry>,
        corpus_stats: CorpusStats,
        next_word: NextWordStats,
        mut log_stage: impl FnMut(&str, f64),
    ) -> Self {
        Self::from_entries_with_stats_stage_logger_and_dictionary_image(
            entries,
            corpus_stats,
            next_word,
            None,
            &mut log_stage,
        )
    }

    pub(crate) fn from_entries_with_stats_stage_logger_and_dictionary_image(
        entries: Vec<Entry>,
        corpus_stats: CorpusStats,
        next_word: NextWordStats,
        dictionary_image: Option<DictionaryImageView<'static>>,
        mut log_stage: impl FnMut(&str, f64),
    ) -> Self {
        let started = start_stage_timer();
        let mut maps = if dictionary_image.is_some() {
            LegacyLookupMaps::roman_keys_only(&entries)
        } else {
            Self::build_lookup_maps(&entries)
        };
        log_stage("build_lookup_maps", elapsed_stage_ms(started));

        let started = start_stage_timer();
        let target_frequency = if dictionary_image.is_some() {
            HashMap::new()
        } else {
            target_frequency_map(&entries, Some(&corpus_stats))
        };
        log_stage("target_frequency", elapsed_stage_ms(started));

        let started = start_stage_timer();
        if dictionary_image.is_none() {
            sort_lookup_maps_by_frequency(&mut maps, &target_frequency);
        }
        log_stage("sort_lookup_maps", elapsed_stage_ms(started));

        let started = start_stage_timer();
        let next_word_max_context_chars = max_next_word_context_chars(&next_word);
        log_stage("next_word_context", elapsed_stage_ms(started));

        let started = start_stage_timer();
        let index_mode = if dictionary_image.is_some() {
            RankedLookupIndexMode::SkipExactAliasAndGram
        } else {
            RankedLookupIndexMode::BuildRetrievalIndexes
        };
        let mut ranked = RankedLexicon::from_entries_with_stage_logger_and_index_mode(
            &entries,
            corpus_stats,
            index_mode,
            |stage, elapsed_ms| {
                log_stage(&format!("ranked_lexicon.{stage}"), elapsed_ms);
            },
        );
        // With the image present the decoder reads entries and count sections from
        // the image, so the corresponding heap tables are redundant.
        if dictionary_image.is_some() {
            ranked.entries = Vec::new();
            ranked.word_unigrams = HashMap::new();
            ranked.word_bigrams = HashMap::new();
            ranked.corpus_word_unigrams = HashMap::new();
            ranked.corpus_word_bigrams = HashMap::new();
            ranked.corpus_surface_unigrams = HashMap::new();
            ranked.tag_unigrams = HashMap::new();
            ranked.tag_bigrams = HashMap::new();
        }
        log_stage("ranked_lexicon", elapsed_stage_ms(started));

        let started = start_stage_timer();
        let index = SearchIndex::new(&maps.roman_keys, true, 2, 3);
        log_stage("search_index", elapsed_stage_ms(started));

        let started = start_stage_timer();
        let data = Self {
            entries,
            by_roman: maps.by_roman,
            by_normalized: maps.by_normalized,
            by_target: maps.by_target,
            target_frequency,
            roman_normalized: maps.roman_normalized,
            roman_prefix_index: maps.roman_prefix_index,
            index,
            ranked,
            dictionary_image,
            next_word,
            next_word_max_context_chars,
        };
        log_stage("assemble", elapsed_stage_ms(started));
        data
    }

    fn build_lookup_maps(entries: &[Entry]) -> LegacyLookupMaps {
        let mut by_roman = HashMap::<String, Vec<String>>::new();
        let mut by_normalized = HashMap::<String, Vec<String>>::new();
        let mut by_target = HashMap::<String, Vec<String>>::new();
        let target_frequency = target_frequency_map(entries, None);
        for entry in entries {
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
        for values in by_roman.values_mut() {
            values.sort_by(|left, right| {
                target_frequency
                    .get(right)
                    .copied()
                    .unwrap_or(1)
                    .cmp(&target_frequency.get(left).copied().unwrap_or(1))
                    .then_with(|| left.cmp(right))
            });
        }
        for values in by_normalized.values_mut() {
            values.sort_by(|left, right| {
                target_frequency
                    .get(right)
                    .copied()
                    .unwrap_or(1)
                    .cmp(&target_frequency.get(left).copied().unwrap_or(1))
                    .then_with(|| left.cmp(right))
            });
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
        LegacyLookupMaps {
            by_roman,
            by_normalized,
            by_target,
            target_frequency,
            roman_normalized,
            roman_prefix_index,
            roman_keys: entries.iter().map(|entry| entry.roman.clone()).collect::<Vec<_>>(),
        }
    }

    pub(crate) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    // A view of ranked entry `entry_id`, sourced from the zero-copy dictionary image
    // when present (default system lexicon) or the heap ranked table otherwise
    // (custom/CLI lexicons). The image and heap entry-id spaces are identical (see
    // dictionary_image_matches_ranked_retrieval_indexes), so callers index the same
    // way regardless of source.
    pub(crate) fn ranked_entry(&self, entry_id: usize) -> RankedEntryView<'_> {
        if let Some(image) = self.dictionary_image {
            let id = entry_id as u32;
            let entry = image.entry(id).expect("dictionary image entry must resolve");
            RankedEntryView::Image {
                image,
                entry,
                entry_id: id,
            }
        } else {
            RankedEntryView::Heap(&self.ranked.entries[entry_id])
        }
    }

    pub(crate) fn starter_suggestions(&self, history: &HashMap<String, usize>) -> Vec<String> {
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

    pub(crate) fn best_prefix_consumption(&self, input: &str, target: &str) -> Option<String> {
        let normalized_input = normalize(input);
        if normalized_input.is_empty() {
            return None;
        }

        let mut matches = self
            .romans_for_target(target)
            .into_iter()
            .filter(|roman| !roman.is_empty() && normalized_input.starts_with(roman.as_str()))
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
        matches.dedup();
        matches.into_iter().next()
    }

    pub(crate) fn exact_match_roman_variants(&self, input: &str, target: &str) -> Vec<String> {
        let query = input.strip_suffix(' ').unwrap_or(input);
        let normalized_query = normalize(query);
        if normalized_query.is_empty() {
            return Vec::new();
        }

        let mut variants = self
            .romans_for_target(target)
            .into_iter()
            .filter(|roman| !roman.is_empty())
            .collect::<Vec<_>>();
        variants.sort_by(|left, right| {
            let left_is_query = left == &normalized_query;
            let right_is_query = right == &normalized_query;
            right_is_query
                .cmp(&left_is_query)
                .then_with(|| left.len().cmp(&right.len()))
                .then_with(|| left.cmp(right))
        });
        variants.dedup();
        variants
    }

    pub(crate) fn suggest(&self, input: &str, history: &HashMap<String, usize>) -> Vec<String> {
        self.suggest_with_limit(input, history, MAX_SUGGESTIONS)
    }

    pub(crate) fn suggest_with_limit(&self, input: &str, history: &HashMap<String, usize>, limit: usize) -> Vec<String> {
        let query = input.strip_suffix(' ').unwrap_or(input);
        if query == "." {
            return vec![
                "។".to_owned(),
                "៕".to_owned(),
                ".".to_owned(),
                "?".to_owned(),
                "!".to_owned(),
                "…".to_owned(),
            ];
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
                .prefix_romans(&prefix_seed)
                .unwrap_or_else(|| self.all_roman_keys());
            let prefix_matches = seed_pool
                .into_iter()
                .filter(|roman| self.normalized_for_roman(roman).starts_with(&normalized))
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
                            frequency: self.target_frequency_for(target),
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
            let exact_match = self.normalized_for_roman(&roman) == normalized;
            let roman_len = roman.chars().count();
            let values = self.targets_for_roman(&roman);
            if !values.is_empty() {
                for target in values {
                    push_candidate(
                        &mut suggestions,
                        &mut seen,
                        &target,
                        CandidateMeta {
                            exact_match,
                            frequency: self.target_frequency_for(&target),
                            target_len: target.chars().count(),
                            roman_len,
                            visit_index,
                        },
                    );
                    visit_index += 1;
                    if suggestions.len() >= limit {
                        break;
                    }
                }
            }
            if suggestions.len() >= limit {
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
        suggestions.truncate(limit);
        append_raw_query_fallback(&mut suggestions, query, limit);
        suggestions
    }

    pub(crate) fn next_word_suggestions(
        &self,
        previous_token: &str,
        sentence_start: bool,
        history: &HashMap<String, usize>,
    ) -> Vec<String> {
        if sentence_start {
            return Vec::new();
        }

        let context_key = map_next_word_context_token(previous_token);
        let mut scores = HashMap::<String, (u32, usize, u32)>::new();
        if let Some(context_rows) = self.next_word.bigrams.get(&context_key) {
            for (word, count) in context_rows {
                let unigram = self.next_word.unigrams.get(word).copied().unwrap_or(0);
                let history_count = history.get(word).copied().unwrap_or(0);
                scores.insert(word.clone(), (*count, history_count, unigram));
            }
        }

        for (word, unigram) in self.next_word.ranked_unigrams.iter().take(NEXT_WORD_BACKFILL_POOL) {
            let history_count = history.get(word).copied().unwrap_or(0);
            scores.entry(word.clone()).or_insert((0, history_count, *unigram));
        }

        let mut ranked = scores.into_iter().collect::<Vec<_>>();
        ranked.sort_by(|(left_word, left), (right_word, right)| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| left_word.cmp(right_word))
        });
        ranked.truncate(MAX_SUGGESTIONS);
        ranked.into_iter().map(|(word, _)| word).collect()
    }

    pub(crate) fn infer_next_word_context_suffix(&self, text_before_caret: &str) -> Option<String> {
        if self.next_word.bigrams.is_empty() || self.next_word_max_context_chars == 0 {
            return None;
        }

        let chars = text_before_caret.chars().collect::<Vec<_>>();
        let end = chars.len();
        if end == 0 {
            return None;
        }

        let max_len = self.next_word_max_context_chars.min(end);
        let min_start = end.saturating_sub(max_len);
        for start in min_start..end {
            let first = chars[start];
            if !is_khmer_char(first) {
                continue;
            }
            let candidate = chars[start..end].iter().collect::<String>();
            if self.next_word.bigrams.contains_key(&candidate) {
                return Some(candidate);
            }
        }
        None
    }

    pub(crate) fn exact_targets(&self, normalized: &str) -> Vec<String> {
        self.targets_for_normalized(normalized)
    }

    fn targets_for_roman(&self, roman: &str) -> Vec<String> {
        if let Some(image) = self.dictionary_image {
            return image
                .legacy_roman_targets(roman)
                .ok()
                .flatten()
                .map(|values| values.into_iter().map(str::to_owned).collect())
                .unwrap_or_default();
        }
        self.by_roman.get(roman).cloned().unwrap_or_default()
    }

    fn targets_for_normalized(&self, normalized: &str) -> Vec<String> {
        if let Some(image) = self.dictionary_image {
            return image
                .legacy_normalized_targets(normalized)
                .ok()
                .flatten()
                .map(|values| values.into_iter().map(str::to_owned).collect())
                .unwrap_or_default();
        }
        self.by_normalized.get(normalized).cloned().unwrap_or_default()
    }

    pub(crate) fn romans_for_target(&self, target: &str) -> Vec<String> {
        if let Some(image) = self.dictionary_image {
            return image
                .legacy_target_romans(target)
                .ok()
                .flatten()
                .map(|values| values.into_iter().map(str::to_owned).collect())
                .unwrap_or_default();
        }
        self.by_target.get(target).cloned().unwrap_or_default()
    }

    pub(crate) fn has_target(&self, target: &str) -> bool {
        !self.romans_for_target(target).is_empty()
    }

    fn prefix_romans(&self, prefix: &str) -> Option<Vec<String>> {
        if let Some(image) = self.dictionary_image {
            return image
                .legacy_prefix_romans(prefix)
                .ok()
                .flatten()
                .map(|values| values.into_iter().map(str::to_owned).collect());
        }
        self.roman_prefix_index.get(prefix).cloned()
    }

    fn all_roman_keys(&self) -> Vec<String> {
        if let Some(image) = self.dictionary_image {
            return image
                .legacy_all_romans()
                .map(|values| values.into_iter().map(str::to_owned).collect())
                .unwrap_or_default();
        }
        self.by_roman.keys().cloned().collect()
    }

    fn normalized_for_roman(&self, roman: &str) -> String {
        if self.dictionary_image.is_some() {
            return normalize(roman);
        }
        self.roman_normalized
            .get(roman)
            .cloned()
            .unwrap_or_else(|| normalize(roman))
    }

    pub(crate) fn target_frequency_for(&self, target: &str) -> u32 {
        if let Some(image) = self.dictionary_image {
            return image.legacy_target_frequency(target).ok().flatten().unwrap_or(1);
        }
        self.target_frequency.get(target).copied().unwrap_or(1)
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub(crate) fn attach_dictionary_image(&mut self, dictionary_image: DictionaryImageView<'static>) {
        self.dictionary_image = Some(dictionary_image);
    }

    pub(crate) fn has_ranked_exact_key(&self, key: &str) -> bool {
        if let Some(image) = self.dictionary_image {
            return image.exact_postings(key).ok().flatten().is_some();
        }
        self.ranked.exact_index.contains_key(key)
    }

    pub(crate) fn ranked_exact_entry_ids(&self, key: &str) -> Vec<usize> {
        if let Some(image) = self.dictionary_image {
            return image
                .exact_postings(key)
                .ok()
                .flatten()
                .map(|ids| ids.map(|id| id as usize).collect())
                .unwrap_or_default();
        }
        self.ranked.exact_index.get(key).cloned().unwrap_or_default()
    }

    pub(crate) fn ranked_alias_entry_ids(&self, key: &str) -> Vec<usize> {
        if let Some(image) = self.dictionary_image {
            return image
                .alias_postings(key)
                .ok()
                .flatten()
                .map(|ids| ids.map(|id| id as usize).collect())
                .unwrap_or_default();
        }
        self.ranked.alias_index.get(key).cloned().unwrap_or_default()
    }

    pub(crate) fn ranked_gram_entry_ids(&self, key: &str) -> Vec<usize> {
        if let Some(image) = self.dictionary_image {
            return image
                .gram_postings(key)
                .ok()
                .flatten()
                .map(|ids| ids.map(|id| id as usize).collect())
                .unwrap_or_default();
        }
        self.ranked.gram_index.get(key).cloned().unwrap_or_default()
    }

    pub(crate) fn word_unigram_count(&self, word: &str) -> u32 {
        if let Some(image) = self.dictionary_image {
            return image
                .word_unigram_count(word)
                .expect("dictionary image word unigram count");
        }
        self.ranked.word_unigrams.get(word).copied().unwrap_or(0)
    }

    pub(crate) fn word_bigram_count(&self, left: &str, right: &str) -> u32 {
        if let Some(image) = self.dictionary_image {
            return image
                .word_bigram_count(left, right)
                .expect("dictionary image word bigram count");
        }
        self.ranked
            .word_bigrams
            .get(&(left.to_owned(), right.to_owned()))
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn corpus_word_unigram_count(&self, word: &str) -> u32 {
        if let Some(image) = self.dictionary_image {
            return image
                .corpus_word_unigram_count(word)
                .expect("dictionary image corpus word unigram count");
        }
        self.ranked.corpus_word_unigrams.get(word).copied().unwrap_or(0)
    }

    pub(crate) fn corpus_word_bigram_count(&self, left: &str, right: &str) -> u32 {
        if let Some(image) = self.dictionary_image {
            return image
                .corpus_word_bigram_count(left, right)
                .expect("dictionary image corpus word bigram count");
        }
        self.ranked
            .corpus_word_bigrams
            .get(&(left.to_owned(), right.to_owned()))
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn corpus_surface_unigram_count(&self, surface: &str) -> u32 {
        if let Some(image) = self.dictionary_image {
            return image
                .corpus_surface_unigram_count(surface)
                .expect("dictionary image corpus surface unigram count");
        }
        self.ranked.corpus_surface_unigrams.get(surface).copied().unwrap_or(0)
    }

    pub(crate) fn tag_unigram_count(&self, tag: &str) -> u32 {
        if let Some(image) = self.dictionary_image {
            return image
                .tag_unigram_count(tag)
                .expect("dictionary image tag unigram count");
        }
        self.ranked.tag_unigrams.get(tag).copied().unwrap_or(0)
    }

    pub(crate) fn tag_bigram_count(&self, left: &str, right: &str) -> u32 {
        if let Some(image) = self.dictionary_image {
            return image
                .tag_bigram_count(left, right)
                .expect("dictionary image tag bigram count");
        }
        self.ranked
            .tag_bigrams
            .get(&(left.to_owned(), right.to_owned()))
            .copied()
            .unwrap_or(0)
    }
}

fn append_raw_query_fallback(suggestions: &mut Vec<String>, query: &str, limit: usize) {
    if !is_raw_query_fallback_token(query) || limit == 0 {
        return;
    }

    suggestions.retain(|item| item != query);
    if suggestions.len() >= limit {
        suggestions.truncate(limit - 1);
    }
    suggestions.push(query.to_owned());
}

fn is_raw_query_fallback_token(query: &str) -> bool {
    !query.is_empty() && query.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn max_next_word_context_chars(next_word: &NextWordStats) -> usize {
    next_word
        .bigrams
        .keys()
        .map(|token| token.chars().count())
        .max()
        .unwrap_or(0)
}

fn is_khmer_char(ch: char) -> bool {
    ('\u{1780}'..='\u{17ff}').contains(&ch) || ('\u{19e0}'..='\u{19ff}').contains(&ch)
}

fn target_frequency_map(entries: &[Entry], corpus_stats: Option<&CorpusStats>) -> HashMap<String, u32> {
    let mut frequency = HashMap::<String, u32>::new();
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

fn sort_lookup_maps_by_frequency(maps: &mut LegacyLookupMaps, target_frequency: &HashMap<String, u32>) {
    for values in maps.by_roman.values_mut() {
        sort_targets_by_frequency(values, target_frequency);
    }
    for values in maps.by_normalized.values_mut() {
        sort_targets_by_frequency(values, target_frequency);
    }
}

fn sort_targets_by_frequency(values: &mut [String], target_frequency: &HashMap<String, u32>) {
    values.sort_by(|left, right| {
        target_frequency
            .get(right)
            .copied()
            .unwrap_or(1)
            .cmp(&target_frequency.get(left).copied().unwrap_or(1))
            .then_with(|| left.cmp(right))
    });
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
    frequency: u32,
    target_len: usize,
    roman_len: usize,
    visit_index: usize,
}

impl CandidateMeta {
    fn better_than(self, other: Self) -> bool {
        self.cmp_priority(other).is_gt()
    }

    fn cmp_priority(self, other: Self) -> std::cmp::Ordering {
        let base = self.exact_match.cmp(&other.exact_match);
        if base != std::cmp::Ordering::Equal {
            return base;
        }
        if self.exact_match {
            return self
                .frequency
                .cmp(&other.frequency)
                .then_with(|| other.target_len.cmp(&self.target_len))
                .then_with(|| other.roman_len.cmp(&self.roman_len))
                .then_with(|| other.visit_index.cmp(&self.visit_index));
        }
        other
            .target_len
            .cmp(&self.target_len)
            .then_with(|| other.roman_len.cmp(&self.roman_len))
            .then_with(|| self.frequency.cmp(&other.frequency))
            .then_with(|| other.visit_index.cmp(&self.visit_index))
    }
}

impl LegacyLookupMaps {
    fn roman_keys_only(entries: &[Entry]) -> Self {
        Self {
            by_roman: HashMap::new(),
            by_normalized: HashMap::new(),
            by_target: HashMap::new(),
            target_frequency: HashMap::new(),
            roman_normalized: HashMap::new(),
            roman_prefix_index: HashMap::new(),
            roman_keys: entries.iter().map(|entry| entry.roman.clone()).collect(),
        }
    }
}
