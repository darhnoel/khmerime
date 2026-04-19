// Khmer normalization logic in this module is adapted primarily from:
// - local `khnormal.js` in this repository
//   (`khnormal.js` states it is a direct translation of
//    https://github.com/sillsdev/khmer-character-specification/blob/master/python/scripts/khnormal)
//
// Upstream attribution:
// Copyright (c) 2021-2024, SIL International.
// Licensed under MIT.
//
// This Rust implementation keeps equivalent normalization intent while adapting
// structure and APIs to this crate.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Cat {
    Other = 0,
    Base = 1,
    Robat = 2,
    Coeng = 3,
    ZfCoeng = 4,
    Shift = 5,
    Z = 6,
    VPre = 7,
    Vb = 8,
    Va = 9,
    VPost = 10,
    Ms = 11,
    Mf = 12,
}

const COENG: char = '\u{17D2}';
const ZWNJ: char = '\u{200C}';
const ZWJ: char = '\u{200D}';
const ROBAT_CONSONANT: char = '\u{179A}';
const DA_CONSONANT: char = '\u{178A}';
const TA_CONSONANT: char = '\u{178F}';

fn in_range(ch: char, start: u32, end: u32) -> bool {
    let code = ch as u32;
    (start..=end).contains(&code)
}

fn char_category(ch: char) -> Cat {
    match ch as u32 {
        0x1780..=0x17A2 => Cat::Base,
        0x17A3..=0x17A4 => Cat::Other,
        0x17A5..=0x17B3 => Cat::Base,
        0x17B4..=0x17B5 => Cat::Other,
        0x17B6 => Cat::VPost,
        0x17B7..=0x17BA => Cat::Va,
        0x17BB..=0x17BD => Cat::Vb,
        0x17BE..=0x17C5 => Cat::VPre,
        0x17C6 => Cat::Ms,
        0x17C7..=0x17C8 => Cat::Mf,
        0x17C9..=0x17CA => Cat::Shift,
        0x17CB => Cat::Ms,
        0x17CC => Cat::Robat,
        0x17CD..=0x17D1 => Cat::Ms,
        0x17D2 => Cat::Coeng,
        0x17D3 => Cat::Ms,
        0x17D4..=0x17DC => Cat::Other,
        0x17DD => Cat::Ms,
        0x200C => Cat::Z,
        0x200D => Cat::ZfCoeng,
        _ => Cat::Other,
    }
}

fn mark_middle_khmer_final_coengs(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(input.len() + 8);
    let mut i = 0usize;
    while i < chars.len() {
        if i + 1 < chars.len() && in_range(chars[i], 0x17B7, 0x17C5) && chars[i + 1] == COENG {
            out.push(chars[i]);
            out.push(chars[i + 1]);
            out.push(ZWJ);
            i += 2;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn is_invisible(ch: char) -> bool {
    ch == COENG || ch == ZWNJ || ch == ZWJ
}

fn collapse_multiple_invisibles(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < chars.len() {
        let mut prefix_len = 0usize;
        if chars[i] == ZWNJ || chars[i] == ZWJ {
            prefix_len = if i + 1 < chars.len() && chars[i + 1] == COENG {
                2
            } else {
                1
            };
        } else if chars[i] == COENG && i + 1 < chars.len() && chars[i + 1] == ZWJ {
            prefix_len = 2;
        }

        if prefix_len == 0 {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        let mut j = i + prefix_len;
        while j < chars.len() && is_invisible(chars[j]) {
            j += 1;
        }

        if j > i + prefix_len {
            for ch in &chars[i..i + prefix_len] {
                out.push(*ch);
            }
            i = j;
            continue;
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

fn replace_compound_vowel_seq(input: &str, target: char, replacement: char) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '\u{17C1}' {
            if i + 1 < chars.len() && chars[i + 1] == target {
                out.push(replacement);
                i += 2;
                continue;
            }
            if i + 2 < chars.len() && in_range(chars[i + 1], 0x17BB, 0x17BD) && chars[i + 2] == target {
                out.push(replacement);
                out.push(chars[i + 1]);
                i += 3;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn swap_u_before_ve(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '\u{17BE}' && chars[i + 1] == '\u{17BB}' {
            out.push(chars[i + 1]);
            out.push(chars[i]);
            i += 2;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn is_strong_series(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1780..=0x1783
            | 0x1785..=0x1788
            | 0x178A..=0x178D
            | 0x178F..=0x1792
            | 0x1795..=0x1797
            | 0x179E..=0x17A0
            | 0x17A2
    )
}

fn is_vaa_ahead(chars: &[char], index: usize) -> bool {
    if index >= chars.len() {
        return false;
    }
    let ch = chars[index];
    if in_range(ch, 0x17B7, 0x17BA) || ch == '\u{17BE}' || ch == '\u{17BF}' || ch == '\u{17DD}' {
        return true;
    }
    ch == '\u{17B6}' && index + 1 < chars.len() && chars[index + 1] == '\u{17C6}'
}

fn replace_u_with_shifter(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(input.len());
    for i in 0..chars.len() {
        if chars[i] == '\u{17BB}'
            && ((i + 1 < chars.len() && is_vaa_ahead(&chars, i + 1))
                || (i + 1 < chars.len() && chars[i + 1] == '\u{17D0}'))
        {
            let mut base = None::<char>;
            let mut j = i;
            while j > 0 {
                j -= 1;
                if in_range(chars[j], 0x1780, 0x17B3) {
                    base = Some(chars[j]);
                    break;
                }
            }
            if base.map(is_strong_series).unwrap_or(false) {
                out.push('\u{17CA}');
            } else {
                out.push('\u{17C9}');
            }
            continue;
        }
        out.push(chars[i]);
    }
    out
}

fn move_coeng_ro_second(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < chars.len() {
        if i + 3 < chars.len()
            && chars[i] == COENG
            && chars[i + 1] == ROBAT_CONSONANT
            && chars[i + 2] == COENG
            && in_range(chars[i + 3], 0x1780, 0x17B3)
        {
            out.push(chars[i + 2]);
            out.push(chars[i + 3]);
            out.push(chars[i]);
            out.push(chars[i + 1]);
            i += 4;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn normalize_coeng_da_to_ta(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == COENG && chars[i + 1] == DA_CONSONANT {
            out.push(COENG);
            out.push(TA_CONSONANT);
            i += 2;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn normalize_syllable(sorted: &str) -> String {
    let mut s = collapse_multiple_invisibles(sorted);
    s = replace_compound_vowel_seq(&s, '\u{17B8}', '\u{17BE}');
    s = replace_compound_vowel_seq(&s, '\u{17B6}', '\u{17C4}');
    s = swap_u_before_ve(&s);
    s = replace_u_with_shifter(&s);
    s = move_coeng_ro_second(&s);
    normalize_coeng_da_to_ta(&s)
}

/// Khmer character normalization adapted from SIL `khnormal`.
///
/// This function performs category-based Khmer syllable reordering and
/// canonical cleanup suitable for lookup/indexing. It intentionally avoids
/// changing non-Khmer text.
pub fn khnormal(text: &str, lang: &str) -> String {
    let prepared = if lang == "xhm" {
        mark_middle_khmer_final_coengs(text)
    } else {
        text.to_owned()
    };
    let chars = prepared.chars().collect::<Vec<_>>();
    let mut categories = chars.iter().map(|ch| char_category(*ch)).collect::<Vec<_>>();

    for i in 1..categories.len() {
        if (chars[i - 1] == COENG || chars[i - 1] == ZWJ)
            && (categories[i] == Cat::Base || categories[i] == Cat::ZfCoeng)
        {
            categories[i] = Cat::Coeng;
        }
    }

    let mut i = 0usize;
    let mut out = String::with_capacity(prepared.len());
    while i < chars.len() {
        if categories[i] != Cat::Base {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        let mut j = i + 1;
        while j < chars.len() && categories[j] > Cat::Base {
            j += 1;
        }

        let mut indices = (i..j).collect::<Vec<_>>();
        indices.sort_by(|a, b| categories[*a].cmp(&categories[*b]).then_with(|| a.cmp(b)));

        let sorted = indices.iter().map(|idx| chars[*idx]).collect::<String>();
        out.push_str(&normalize_syllable(&sorted));
        i = j;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::khnormal;

    #[test]
    fn keeps_non_khmer_text_unchanged() {
        assert_eq!(khnormal("hello 123", "km"), "hello 123");
    }

    #[test]
    fn normalizes_coeng_da_to_ta() {
        assert_eq!(khnormal("ក្ត", "km"), "ក្ត");
        assert_eq!(khnormal("ក្ដ", "km"), "ក្ត");
    }

    #[test]
    fn normalizes_compound_vowel_sequences() {
        assert_eq!(khnormal("កេុី", "km"), "ក៊ើ");
        assert_eq!(khnormal("កេុា", "km"), "កោុ");
    }

    #[test]
    fn marks_middle_khmer_final_coengs_for_xhm() {
        assert_eq!(khnormal("ិ្", "xhm"), "ិ្\u{200D}");
    }
}
