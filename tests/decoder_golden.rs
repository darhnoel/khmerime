use std::{fs, path::PathBuf};

use roman_lookup::{DecoderConfig, DecoderMode, Transliterator};

struct GoldenCase {
    name: &'static str,
    input: &'static str,
    top_n: usize,
}

struct GoldenSuite {
    mode: DecoderMode,
    cases: &'static [GoldenCase],
}

const WFST_CASES: &[GoldenCase] = &[
    GoldenCase {
        name: "exact_jea",
        input: "jea",
        top_n: 1,
    },
    GoldenCase {
        name: "exact_ttov",
        input: "ttov",
        top_n: 2,
    },
    GoldenCase {
        name: "single_span_sronos",
        input: "sronos",
        top_n: 7,
    },
    GoldenCase {
        name: "exact_phrase_khnhomttov",
        input: "khnhomttov",
        top_n: 6,
    },
    GoldenCase {
        name: "beam_phrase_saensronors",
        input: "saensronors",
        top_n: 1,
    },
    GoldenCase {
        name: "beam_phrase_knhhomttovsalarien",
        input: "knhhomttovsalarien",
        top_n: 1,
    },
    GoldenCase {
        name: "beam_phrase_khomtaekitmnakaeng",
        input: "khomtaekitmnakaeng",
        top_n: 1,
    },
];

#[test]
fn wfst_suggestions_match_locked_golden_snapshot() {
    let suite = GoldenSuite {
        mode: DecoderMode::Wfst,
        cases: WFST_CASES,
    };
    let transliterator =
        Transliterator::from_default_data_with_config(DecoderConfig::default().with_mode(suite.mode)).unwrap();
    let actual = render_suite(&suite, &transliterator);
    let expected = fs::read_to_string(snapshot_path()).expect("golden snapshot must be readable");

    assert_eq!(
        actual,
        expected,
        "decoder golden mismatch\n\
         protected surface: Transliterator::suggest(...) in Wfst mode\n\
         snapshot: {}\n\
         intentional updates must edit the checked-in golden file explicitly",
        snapshot_path().display()
    );
}

fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/decoder_wfst_suggest.txt")
}

fn render_suite(suite: &GoldenSuite, transliterator: &Transliterator) -> String {
    let mut output = String::new();
    output.push_str("# Locked golden snapshot for Transliterator::suggest in Wfst mode.\n");
    output.push_str("# This file is verified by cargo test and must be updated explicitly.\n");
    output.push_str("mode: wfst\n\n");

    for case in suite.cases {
        output.push_str(&render_case(case, transliterator));
    }

    output
}

fn render_case(case: &GoldenCase, transliterator: &Transliterator) -> String {
    let suggestions = transliterator.suggest(case.input, &std::collections::HashMap::new());
    let mut output = String::new();
    output.push_str("[[case]]\n");
    output.push_str(&format!("name: {}\n", case.name));
    output.push_str(&format!("input: {}\n", case.input));
    output.push_str(&format!("top_n: {}\n", case.top_n));
    output.push_str("suggestions:\n");

    for (index, suggestion) in suggestions.iter().take(case.top_n).enumerate() {
        output.push_str(&format!("{}. {}\n", index + 1, suggestion));
    }

    output.push('\n');
    output
}
