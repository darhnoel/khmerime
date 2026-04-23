use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;

#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
use std::fs;
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
use std::path::Path;

use crate::composer::ComposerTable;
use crate::decoder::{DecoderConfig, DecoderManager, DecoderMode, LegacyDecoder, ShadowObservation, WfstDecoder};

mod compiled_io;
mod legacy_data;
mod normalization;
mod ranked_lexicon;
mod search_index;
mod transliterator;
mod types;

use compiled_io::{parse_compiled_khpos_stats, parse_compiled_lexicon, parse_compiled_next_word_stats};
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
use compiled_io::{parse_csv, parse_tsv};
use normalization::map_next_word_context_token;
use types::*;

pub use types::{AppliedSuggestion, Entry, LexiconError, Result, Transliterator};

pub(crate) use normalization::{char_ngrams, normalize, roman_search_variants};
pub(crate) use types::{LegacyData, RankedLexicon, RankedLexiconEntry};

#[cfg(test)]
mod tests {
    use super::*;
    const DEFAULT_DATA_CSV: &str = include_str!("../../../../data/roman_lookup.csv");

    fn compile_test_lexicon(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(COMPILED_MAGIC);
        bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (roman, target) in entries {
            bytes.extend_from_slice(roman.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(target.as_bytes());
            bytes.push(0);
        }
        bytes
    }

    fn test_next_word_stats(unigrams: &[(&str, u32)], bigrams: &[(&str, &str, u32)]) -> NextWordStats {
        let unigram_map = unigrams
            .iter()
            .map(|(word, count)| ((*word).to_owned(), *count))
            .collect::<HashMap<_, _>>();
        let mut ranked_unigrams = unigram_map
            .iter()
            .map(|(word, count)| (word.clone(), *count))
            .collect::<Vec<_>>();
        ranked_unigrams.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

        let mut bigram_map = HashMap::<String, Vec<(String, u32)>>::new();
        for (left, right, count) in bigrams {
            bigram_map
                .entry((*left).to_owned())
                .or_default()
                .push(((*right).to_owned(), *count));
        }
        for rows in bigram_map.values_mut() {
            rows.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        }

        NextWordStats {
            unigrams: unigram_map,
            bigrams: bigram_map,
            ranked_unigrams,
        }
    }

    fn transliterator_with_next_word(entries: &[(&str, &str)], next_word: NextWordStats) -> Transliterator {
        let entries = entries
            .iter()
            .map(|(roman, target)| Entry {
                roman: (*roman).to_owned(),
                target: (*target).to_owned(),
            })
            .collect::<Vec<_>>();
        let legacy = Arc::new(LegacyData::from_entries_with_stats(
            entries,
            CorpusStats::default(),
            next_word,
        ));
        let composer = ComposerTable::from_entries(legacy.entries());
        let config = DecoderConfig::default();
        let decoder = DecoderManager::new(
            composer,
            LegacyDecoder::new(Arc::clone(&legacy)),
            (config.mode != crate::decoder::DecoderMode::Legacy)
                .then(|| WfstDecoder::new(Arc::clone(&legacy), config.clone())),
            config,
        );
        Transliterator { legacy, decoder }
    }

    #[test]
    fn loads_embedded_data() {
        let transliterator = Transliterator::from_default_data().unwrap();
        let expected_entries = parse_csv(DEFAULT_DATA_CSV).unwrap().len();
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
    fn compiled_lexicon_matches_csv_source() {
        let compiled_entries = parse_compiled_lexicon(DEFAULT_COMPILED_DATA).unwrap();
        let csv_entries = parse_csv(DEFAULT_DATA_CSV).unwrap();
        assert_eq!(compiled_entries, csv_entries);
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
            vec!["ជា", "ជះ", "ជាត", "ជាម", "ឈាម", "ជាយ", "ជាល", "ជាវ", "ជាស", "ជាង"]
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
            vec!["ទេ", "តែ", "តើ", "តិះ", "តេត", "តេន", "តិច", "តេជ", "តែង", "ទៀត"]
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
    fn phase_a_compiled_lexicon_produces_legacy_suggestions() {
        let compiled = compile_test_lexicon(&[("jea", "ជា"), ("jeat", "ជាត"), ("jeam", "ជាម")]);
        let transliterator = Transliterator::from_compiled_lexicon_bytes(&compiled, DecoderConfig::default()).unwrap();
        let suggestions = transliterator.suggest("jea", &HashMap::new());
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0], "ជា");
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
    fn exact_match_roman_variants_for_target_include_aliases() {
        let fixture = "jea\tជា\nchea\tជា\njear\tជារ\n";
        let transliterator = Transliterator::from_tsv_str(fixture).unwrap();
        assert_eq!(
            transliterator.exact_match_roman_variants("jea", "ជា"),
            vec!["jea".to_owned(), "chea".to_owned()]
        );
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

    #[test]
    fn next_word_context_mapping_matches_marker_rules() {
        assert_eq!(map_next_word_context_token(""), "<oth>");
        assert_eq!(map_next_word_context_token("123"), "<num>");
        assert_eq!(map_next_word_context_token("៨៩"), "<num>");
        assert_eq!(map_next_word_context_token("hello"), "<oth>");
        assert_eq!(map_next_word_context_token("<s>"), "<s>");
        assert_eq!(map_next_word_context_token("ខ្មែរ"), "ខ្មែរ");
    }

    #[test]
    fn next_word_suggestions_prioritize_context_then_history_then_unigram() {
        let transliterator = transliterator_with_next_word(
            &[("khmer", "ខ្មែរ")],
            test_next_word_stats(
                &[("ជា", 80), ("ក្នុង", 70), ("នេះ", 60)],
                &[("ខ្មែរ", "ជា", 12), ("ខ្មែរ", "ក្នុង", 12)],
            ),
        );
        let mut history = HashMap::new();
        history.insert("ក្នុង".to_owned(), 3);

        let suggestions = transliterator.next_word_suggestions("ខ្មែរ", false, &history);
        assert_eq!(suggestions.first().map(String::as_str), Some("ក្នុង"));
        assert_eq!(suggestions.get(1).map(String::as_str), Some("ជា"));
        assert!(suggestions.iter().any(|word| word == "នេះ"));
    }

    #[test]
    fn next_word_suggestions_suppress_sentence_start() {
        let transliterator = transliterator_with_next_word(
            &[("khmer", "ខ្មែរ")],
            test_next_word_stats(&[("ជា", 10)], &[("<s>", "ជា", 20)]),
        );
        let suggestions = transliterator.next_word_suggestions("<s>", true, &HashMap::new());
        assert!(suggestions.is_empty());
    }

    #[test]
    fn infer_next_word_context_suffix_extracts_tail_from_concatenated_khmer() {
        let transliterator = transliterator_with_next_word(
            &[("khmer", "ខ្មែរ")],
            test_next_word_stats(&[("ធ្វើ", 10)], &[("ទៅ", "លើ", 25), ("ការ", "ធ្វើ", 20), ("ខ្ញុំ", "ទៅ", 15)]),
        );
        assert_eq!(
            transliterator.infer_next_word_context_suffix("ខ្ញុំទៅ"),
            Some("ទៅ".to_owned())
        );
        assert_eq!(
            transliterator.infer_next_word_context_suffix("គាត់ធ្វើការ"),
            Some("ការ".to_owned())
        );
        assert_eq!(transliterator.infer_next_word_context_suffix("abc"), None);
    }
}
