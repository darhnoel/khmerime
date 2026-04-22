use super::*;

pub(crate) fn map_next_word_context_token(previous_token: &str) -> String {
    let token = previous_token.trim();
    if token.is_empty() {
        return "<oth>".to_owned();
    }
    if matches!(token, "<s>" | "<s> <s>") {
        return token.to_owned();
    }
    let Some(first) = token.chars().next() else {
        return "<oth>".to_owned();
    };
    if first.is_ascii_digit() || ('០'..='៩').contains(&first) {
        return "<num>".to_owned();
    }
    if is_khmer_scalar(first) {
        return token.to_owned();
    }
    "<oth>".to_owned()
}

fn is_khmer_scalar(ch: char) -> bool {
    ('\u{1780}'..='\u{17ff}').contains(&ch) || ('\u{19e0}'..='\u{19ff}').contains(&ch)
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
