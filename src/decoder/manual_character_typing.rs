use std::collections::HashSet;
use std::sync::OnceLock;

const CHARACTER_RELATION_CSV: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/khmer_character_relation.csv"
));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManualComposeKind {
    BaseConsonant,
    Vowel,
    Subscript,
}

impl ManualComposeKind {
    pub fn label(self) -> &'static str {
        match self {
            ManualComposeKind::BaseConsonant => "base consonant",
            ManualComposeKind::Vowel => "vowel",
            ManualComposeKind::Subscript => "subscript",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManualComposeCandidate {
    pub roman_span: String,
    pub kind: ManualComposeKind,
    pub display_text: String,
    pub insert_text: String,
    pub score: i32,
}

#[derive(Clone, Debug)]
struct CharacterRelationEntry {
    text: String,
    kind: ManualComposeKind,
    patterns: Vec<String>,
}

pub fn suggest_manual_character_candidates(
    remaining_roman: &str,
    expected_kind: ManualComposeKind,
    limit: usize,
) -> Vec<ManualComposeCandidate> {
    if limit == 0 {
        return Vec::new();
    }
    let normalized = normalize_roman_piece(remaining_roman);
    if normalized.is_empty() {
        return Vec::new();
    }

    let expanded_limit = limit.max(32);
    let mut merged = Vec::new();

    for kind in candidate_kind_order(expected_kind) {
        let starter = starter_candidates_for_seed(&normalized, kind);
        let ranked = collect_candidates_for_kind(&normalized, kind, expanded_limit);
        // Keep kind-local starter hints ahead of ranked matches.
        append_deduped_candidates(&mut merged, starter, expanded_limit);
        append_deduped_candidates(&mut merged, ranked, expanded_limit);
    }
    rerank_manual_candidates(&normalized, expected_kind, &mut merged);
    merged
}

fn candidate_kind_order(expected_kind: ManualComposeKind) -> [ManualComposeKind; 3] {
    match expected_kind {
        ManualComposeKind::BaseConsonant => [
            ManualComposeKind::BaseConsonant,
            ManualComposeKind::Vowel,
            ManualComposeKind::Subscript,
        ],
        ManualComposeKind::Vowel => [
            ManualComposeKind::Vowel,
            ManualComposeKind::Subscript,
            ManualComposeKind::BaseConsonant,
        ],
        ManualComposeKind::Subscript => [
            ManualComposeKind::Subscript,
            ManualComposeKind::Vowel,
            ManualComposeKind::BaseConsonant,
        ],
    }
}

fn rerank_manual_candidates(
    seed: &str,
    expected_kind: ManualComposeKind,
    candidates: &mut Vec<ManualComposeCandidate>,
) {
    let kind_rank = |kind: ManualComposeKind| -> i32 {
        if kind == expected_kind {
            0
        } else {
            match (expected_kind, kind) {
                (ManualComposeKind::BaseConsonant, ManualComposeKind::Vowel)
                | (ManualComposeKind::Vowel, ManualComposeKind::Subscript)
                | (ManualComposeKind::Subscript, ManualComposeKind::Vowel) => 1,
                _ => 2,
            }
        }
    };

    candidates.sort_by(|left, right| {
        let left_rank = kind_rank(left.kind);
        let right_rank = kind_rank(right.kind);
        let left_score = left.score + rerank_context_bonus(seed, left);
        let right_score = right.score + rerank_context_bonus(seed, right);

        left_rank
            .cmp(&right_rank)
            .then_with(|| right_score.cmp(&left_score))
            .then_with(|| right.roman_span.len().cmp(&left.roman_span.len()))
            .then_with(|| left.insert_text.cmp(&right.insert_text))
    });
}

fn rerank_context_bonus(seed: &str, candidate: &ManualComposeCandidate) -> i32 {
    let mut bonus = 0;

    if candidate.roman_span.is_empty() {
        // Generic starter hints should not outrank concrete roman matches.
        bonus -= 24;
    }

    if candidate.kind == ManualComposeKind::BaseConsonant
        && starts_with_roman_vowel(seed)
        && candidate.insert_text == "អ"
    {
        bonus += 80;
    }

    if candidate.kind == ManualComposeKind::Vowel
        && candidate.insert_text == "័"
        && seed
            .chars()
            .next()
            .is_some_and(|ch| matches!(ch, 'n' | 's' | 'm' | 'l' | 'g' | 'p' | 'h' | 't' | 'v' | 'b' | 'c'))
    {
        bonus += 120;
    }

    bonus
}

fn collect_candidates_for_kind(
    normalized_input: &str,
    kind: ManualComposeKind,
    limit: usize,
) -> Vec<ManualComposeCandidate> {
    let mut ranked = relation_entries()
        .iter()
        .filter(|entry| entry.kind == kind)
        .filter_map(|entry| {
            best_matching_pattern(&entry.patterns, normalized_input).map(|matched| ManualComposeCandidate {
                roman_span: matched.to_owned(),
                kind,
                display_text: entry.text.clone(),
                insert_text: entry.text.clone(),
                score: (matched.chars().count() as i32) * 100 + relation_seed_bonus(normalized_input, matched),
            })
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.roman_span.len().cmp(&left.roman_span.len()))
            .then_with(|| left.insert_text.cmp(&right.insert_text))
    });

    let mut seen = HashSet::<String>::new();
    let mut deduped = Vec::new();
    for candidate in ranked {
        if seen.insert(candidate.insert_text.clone()) {
            deduped.push(candidate);
            if deduped.len() >= limit {
                break;
            }
        }
    }
    deduped
}

fn relation_seed_bonus(seed: &str, matched: &str) -> i32 {
    if seed == matched {
        return 40;
    }
    if matched.starts_with(seed) {
        return 20;
    }
    0
}

fn best_matching_pattern<'a>(patterns: &'a [String], normalized_input: &str) -> Option<&'a str> {
    patterns
        .iter()
        .filter(|pattern| !pattern.is_empty() && normalized_input.starts_with(pattern.as_str()))
        .max_by(|left, right| left.chars().count().cmp(&right.chars().count()))
        .map(String::as_str)
}

fn relation_entries() -> &'static [CharacterRelationEntry] {
    static ENTRIES: OnceLock<Vec<CharacterRelationEntry>> = OnceLock::new();
    ENTRIES.get_or_init(parse_character_relation_entries).as_slice()
}

fn parse_character_relation_entries() -> Vec<CharacterRelationEntry> {
    let mut parsed = Vec::new();
    for line in CHARACTER_RELATION_CSV.lines().skip(1) {
        let Some((raw_text, raw_relation)) = line.split_once(',') else {
            continue;
        };
        let text = raw_text.trim();
        if text.is_empty() {
            continue;
        }

        let Some(kind) = classify_text_kind(text) else {
            continue;
        };
        let patterns = parse_relation_patterns(raw_relation);
        if patterns.is_empty() {
            continue;
        }
        parsed.push(CharacterRelationEntry {
            text: text.to_owned(),
            kind,
            patterns,
        });
    }
    parsed
}

fn parse_relation_patterns(raw_relation: &str) -> Vec<String> {
    let relation = raw_relation.trim();
    if !relation.starts_with('[') || !relation.ends_with(']') {
        return Vec::new();
    }
    let inner = &relation[1..relation.len().saturating_sub(1)];
    let tokens = inner
        .split(',')
        .map(|token| normalize_roman_piece(token))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    relation_tokens_to_patterns(&tokens)
}

fn relation_tokens_to_patterns(tokens: &[String]) -> Vec<String> {
    let mut patterns = Vec::new();
    let mut seen = HashSet::<String>::new();

    for token in tokens {
        if seen.insert(token.clone()) {
            patterns.push(token.clone());
        }
    }

    let mut cumulative = String::new();
    for token in tokens {
        cumulative.push_str(token);
        if cumulative.chars().count() > 4 {
            break;
        }
        if seen.insert(cumulative.clone()) {
            patterns.push(cumulative.clone());
        }
    }

    for pair in tokens.windows(2) {
        let pair_joined = format!("{}{}", pair[0], pair[1]);
        if pair_joined.chars().count() <= 4 && seen.insert(pair_joined.clone()) {
            patterns.push(pair_joined);
        }
    }

    patterns
}

fn classify_text_kind(text: &str) -> Option<ManualComposeKind> {
    let chars = text.chars().collect::<Vec<_>>();
    let first = *chars.first()?;
    if is_subscript_text(&chars) {
        return Some(ManualComposeKind::Subscript);
    }
    if is_base_consonant(first) {
        return Some(ManualComposeKind::BaseConsonant);
    }
    if chars.iter().all(|ch| is_vowel_or_sign(*ch)) {
        return Some(ManualComposeKind::Vowel);
    }
    None
}

fn is_base_consonant(ch: char) -> bool {
    ('\u{1780}'..='\u{17A2}').contains(&ch)
}

fn is_subscript_text(chars: &[char]) -> bool {
    chars.first() == Some(&'\u{17D2}') && chars.get(1).is_some_and(|ch| is_base_consonant(*ch))
}

fn is_vowel(ch: char) -> bool {
    ('\u{17A3}'..='\u{17B3}').contains(&ch) || ('\u{17B6}'..='\u{17C5}').contains(&ch)
}

fn is_vowel_or_sign(ch: char) -> bool {
    is_vowel(ch) || ('\u{17C6}'..='\u{17D3}').contains(&ch)
}

fn append_deduped_candidates(
    output: &mut Vec<ManualComposeCandidate>,
    incoming: Vec<ManualComposeCandidate>,
    limit: usize,
) {
    let mut seen = output
        .iter()
        .map(|candidate| candidate.insert_text.clone())
        .collect::<HashSet<_>>();
    for candidate in incoming {
        if output.len() >= limit {
            break;
        }
        if seen.insert(candidate.insert_text.clone()) {
            output.push(candidate);
        }
    }
}

fn starter_candidates_for_seed(seed: &str, kind: ManualComposeKind) -> Vec<ManualComposeCandidate> {
    let mut starter = Vec::new();

    if kind == ManualComposeKind::BaseConsonant {
        starter.extend(base_carrier_starters_for_seed(seed));
    }

    if kind == ManualComposeKind::Vowel {
        if starts_with_roman_vowel(seed) {
            starter.extend(generic_vowel_manual_starters());
        } else {
            starter.extend(consonant_followup_vowel_starters());
        }
    }

    if kind == ManualComposeKind::Vowel && seed == "a" {
        starter.extend(["ា", "ោ", "ៅ", "ោះ", "ះ", "ែ", "ៃ", "េ", "ឯ", "អា"].iter().map(|text| {
            ManualComposeCandidate {
                roman_span: "a".to_owned(),
                kind: ManualComposeKind::Vowel,
                display_text: (*text).to_owned(),
                insert_text: (*text).to_owned(),
                score: 10,
            }
        }));
    }

    starter
}

fn generic_vowel_manual_starters() -> Vec<ManualComposeCandidate> {
    ["ា", "ិ", "ី", "ឹ", "ឺ", "ុ", "ូ", "ួ", "េ", "ែ", "ៃ", "ោ", "ៅ", "ោះ", "ះ"]
        .iter()
        .map(|text| ManualComposeCandidate {
            roman_span: String::new(),
            kind: ManualComposeKind::Vowel,
            display_text: (*text).to_owned(),
            insert_text: (*text).to_owned(),
            score: 6,
        })
        .collect()
}

fn consonant_followup_vowel_starters() -> Vec<ManualComposeCandidate> {
    ["ឹ", "ិ", "ី", "ុ", "ូ", "៊"]
        .iter()
        .map(|text| ManualComposeCandidate {
            roman_span: String::new(),
            kind: ManualComposeKind::Vowel,
            display_text: (*text).to_owned(),
            insert_text: (*text).to_owned(),
            score: 7,
        })
        .collect()
}

fn starts_with_roman_vowel(seed: &str) -> bool {
    leading_roman_vowel(seed).is_some()
}

fn leading_roman_vowel(seed: &str) -> Option<&'static str> {
    match seed.chars().next() {
        Some('a') => Some("a"),
        Some('e') => Some("e"),
        Some('i') => Some("i"),
        Some('o') => Some("o"),
        Some('u') => Some("u"),
        _ => None,
    }
}

fn base_carrier_starters_for_seed(seed: &str) -> Vec<ManualComposeCandidate> {
    let Some(vowel_seed) = leading_roman_vowel(seed) else {
        return Vec::new();
    };
    let mut seen = HashSet::<String>::new();
    let mut starters = relation_entries()
        .iter()
        .filter(|entry| entry.kind == ManualComposeKind::BaseConsonant)
        .filter(|entry| entry.patterns.iter().any(|pattern| pattern == vowel_seed))
        .filter_map(|entry| {
            if !seen.insert(entry.text.clone()) {
                return None;
            }
            let score = if entry.text == "អ" { 40 } else { 14 };
            Some(ManualComposeCandidate {
                roman_span: vowel_seed.to_owned(),
                kind: ManualComposeKind::BaseConsonant,
                display_text: entry.text.clone(),
                insert_text: entry.text.clone(),
                score,
            })
        })
        .collect::<Vec<_>>();
    starters.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.insert_text.cmp(&right.insert_text))
    });
    starters
}

fn normalize_roman_piece(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                Some(ch.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect()
}
