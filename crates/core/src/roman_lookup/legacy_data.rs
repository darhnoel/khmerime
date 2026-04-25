use super::search_index::SearchIndex;
use super::*;

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
            roman_normalized: maps.roman_normalized,
            roman_prefix_index: maps.roman_prefix_index,
            // Phase A avoids building the heavyweight fuzzy gram index.
            index: SearchIndex::new(&[], true, 2, 3),
            // Phase A avoids khPOS-derived ranking structures.
            ranked: RankedLexicon::default(),
            // Phase A defers next-word n-gram stats until full engine promotion.
            next_word: NextWordStats::default(),
            next_word_max_context_chars: 0,
        }
    }

    pub(crate) fn from_entries_with_stats(
        entries: Vec<Entry>,
        corpus_stats: CorpusStats,
        next_word: NextWordStats,
    ) -> Self {
        let maps = Self::build_lookup_maps(&entries);
        let next_word_max_context_chars = max_next_word_context_chars(&next_word);
        let ranked = RankedLexicon::from_entries(&entries, &corpus_stats);
        Self {
            entries,
            by_roman: maps.by_roman,
            by_normalized: maps.by_normalized,
            by_target: maps.by_target,
            roman_normalized: maps.roman_normalized,
            roman_prefix_index: maps.roman_prefix_index,
            index: SearchIndex::new(&maps.roman_keys, true, 2, 3),
            ranked,
            next_word,
            next_word_max_context_chars,
        }
    }

    fn build_lookup_maps(entries: &[Entry]) -> LegacyLookupMaps {
        let mut by_roman = HashMap::<String, Vec<String>>::new();
        let mut by_normalized = HashMap::<String, Vec<String>>::new();
        let mut by_target = HashMap::<String, Vec<String>>::new();
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
            roman_normalized,
            roman_prefix_index,
            roman_keys: entries.iter().map(|entry| entry.roman.clone()).collect::<Vec<_>>(),
        }
    }

    pub(crate) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub(crate) fn ranked(&self) -> &RankedLexicon {
        &self.ranked
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

    pub(crate) fn exact_match_roman_variants(&self, input: &str, target: &str) -> Vec<String> {
        let query = input.strip_suffix(' ').unwrap_or(input);
        let normalized_query = normalize(query);
        if normalized_query.is_empty() {
            return Vec::new();
        }

        let Some(romans) = self.by_target.get(target) else {
            return Vec::new();
        };

        let mut variants = romans
            .iter()
            .filter(|roman| !roman.is_empty())
            .cloned()
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
        append_raw_query_fallback(&mut suggestions, query);
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

    pub(crate) fn exact_targets(&self, normalized: &str) -> Option<&[String]> {
        self.by_normalized.get(normalized).map(Vec::as_slice)
    }
}

fn append_raw_query_fallback(suggestions: &mut Vec<String>, query: &str) {
    if !is_raw_query_fallback_token(query) || MAX_SUGGESTIONS == 0 {
        return;
    }

    suggestions.retain(|item| item != query);
    if suggestions.len() >= MAX_SUGGESTIONS {
        suggestions.truncate(MAX_SUGGESTIONS - 1);
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
