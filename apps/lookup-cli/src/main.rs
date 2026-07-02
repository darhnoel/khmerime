//! Developer CLI for inspecting KhmerIME data and decoder behavior.
//!
//! This binary is diagnostic tooling: stats, suggestions, and shadow evaluation.
//! It may call `khmerime_core` directly because it is not a platform IME runtime.

use std::collections::HashMap;
use std::env;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::process;

use roman_lookup::{
    DecoderConfig, DecoderMode, ImeSession, ImeSessionOptions, SegmentedPreviewMode, ShadowObservation, ShadowSummary,
    SpanProposalMode, Transliterator,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(2);
    }

    let mut index = 1;
    let mut data_path = None::<String>;
    let mut output_path = None::<String>;
    let mut config = DecoderConfig::default();
    let mut emit_shadow_rows = false;

    while index < args.len() {
        match args[index].as_str() {
            "--data" => {
                let Some(path) = args.get(index + 1) else {
                    print_usage(&args[0]);
                    process::exit(2);
                };
                data_path = Some(path.clone());
                index += 2;
            }
            "--decoder-mode" => {
                let Some(mode) = args.get(index + 1) else {
                    print_usage(&args[0]);
                    process::exit(2);
                };
                config.mode = parse_decoder_mode(mode).unwrap_or_else(|| {
                    eprintln!("invalid decoder mode: {}", mode);
                    process::exit(2);
                });
                index += 2;
            }
            "--shadow-log" => {
                config.shadow_log = true;
                index += 1;
            }
            "--shadow-sample-bps" => {
                let Some(raw) = args.get(index + 1) else {
                    print_usage(&args[0]);
                    process::exit(2);
                };
                config.shadow_sample_bps =
                    raw.parse::<u16>()
                        .ok()
                        .filter(|value| *value <= 10_000)
                        .unwrap_or_else(|| {
                            eprintln!("invalid shadow sample bps: {}", raw);
                            process::exit(2);
                        });
                index += 2;
            }
            "--span-proposals" => {
                let Some(mode) = args.get(index + 1) else {
                    print_usage(&args[0]);
                    process::exit(2);
                };
                config.span_proposal_mode = parse_span_proposal_mode(mode).unwrap_or_else(|| {
                    eprintln!("invalid span proposal mode: {}", mode);
                    process::exit(2);
                });
                index += 2;
            }
            "--emit-shadow-rows" => {
                emit_shadow_rows = true;
                index += 1;
            }
            "--output" => {
                let Some(path) = args.get(index + 1) else {
                    print_usage(&args[0]);
                    process::exit(2);
                };
                output_path = Some(path.clone());
                index += 2;
            }
            _ => break,
        }
    }

    let Some(command) = args.get(index) else {
        print_usage(&args[0]);
        process::exit(2);
    };
    index += 1;

    match command.as_str() {
        "stats" => {
            let transliterator = load_transliterator(data_path, config.clone())?;
            println!("entries: {}", transliterator.entries().len());
        }
        "suggest" => {
            let Some(query) = args.get(index) else {
                print_usage(&args[0]);
                process::exit(2);
            };
            let transliterator = load_transliterator(data_path, config.clone())?;
            let history = HashMap::new();
            for suggestion in transliterator.suggest(query, &history) {
                println!("{}", suggestion);
            }
        }
        "segmented" => {
            let Some(query) = args.get(index) else {
                print_usage(&args[0]);
                process::exit(2);
            };
            run_segmented_dump(data_path, config, query)?;
        }
        "shadow-eval" => {
            let Some(path) = args.get(index) else {
                print_usage(&args[0]);
                process::exit(2);
            };
            let transliterator = load_transliterator(data_path, config.clone())?;
            run_shadow_eval(&transliterator, path, emit_shadow_rows, output_path.as_deref())?;
        }
        _ => {
            print_usage(&args[0]);
            process::exit(2);
        }
    }

    Ok(())
}

fn load_transliterator(
    data_path: Option<String>,
    config: DecoderConfig,
) -> Result<Transliterator, Box<dyn std::error::Error>> {
    if let Some(path) = data_path {
        Ok(Transliterator::from_data_path_with_config(path, config)?)
    } else {
        Ok(Transliterator::from_default_data_with_config(config)?)
    }
}

fn print_usage(bin: &str) {
    eprintln!("Usage:");
    eprintln!(
        "  {} [--data <path/to/data.csv|data.tsv>] [--decoder-mode legacy|shadow|weighted-span|wfst|hybrid] [--shadow-log] [--shadow-sample-bps 0..10000] [--span-proposals disabled|static-test] stats",
        bin
    );
    eprintln!(
        "  {} [--data <path/to/data.csv|data.tsv>] [--decoder-mode legacy|shadow|weighted-span|wfst|hybrid] [--shadow-log] [--shadow-sample-bps 0..10000] [--span-proposals disabled|static-test] suggest <roman>",
        bin
    );
    eprintln!(
        "  {} [--data <path/to/data.csv|data.tsv>] [--decoder-mode legacy|shadow|weighted-span|wfst|hybrid] [--span-proposals disabled|static-test] [--emit-shadow-rows] [--output <report.txt>] shadow-eval <queries.txt>",
        bin
    );
    eprintln!(
        "  {} [--data <path/to/data.csv|data.tsv>] [--span-proposals disabled|static-test] segmented <roman>",
        bin
    );
}

fn parse_decoder_mode(value: &str) -> Option<DecoderMode> {
    match value {
        "legacy" => Some(DecoderMode::Legacy),
        "shadow" => Some(DecoderMode::Shadow),
        "weighted-span" | "weighted_span" | "wfst" => Some(DecoderMode::Wfst),
        "hybrid" => Some(DecoderMode::Hybrid),
        _ => None,
    }
}

fn parse_span_proposal_mode(value: &str) -> Option<SpanProposalMode> {
    match value {
        "disabled" | "none" | "off" => Some(SpanProposalMode::Disabled),
        "static-test" | "static_test" | "static" => Some(SpanProposalMode::StaticTest),
        "model" => Some(SpanProposalMode::Model),
        _ => None,
    }
}

fn run_segmented_dump(
    data_path: Option<String>,
    config: DecoderConfig,
    roman: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Match the ibus bridge's full-engine configuration.
    let live_mode = if config.mode == DecoderMode::Legacy {
        DecoderMode::Shadow
    } else {
        config.mode
    };
    let live = load_transliterator(
        data_path.clone(),
        DecoderConfig::shadow_interactive()
            .with_mode(live_mode)
            .with_span_proposal_mode(config.span_proposal_mode),
    )?;
    let mut visible_config = DecoderConfig::shadow_interactive().with_mode(DecoderMode::Hybrid);
    visible_config.span_proposal_mode = config.span_proposal_mode;
    visible_config.wfst_max_latency_ms = 75;
    let visible_refiner = load_transliterator(data_path.clone(), visible_config)?;
    let mut commit_config = DecoderConfig::default()
        .with_mode(DecoderMode::Hybrid)
        .with_shadow_log(false);
    commit_config.span_proposal_mode = config.span_proposal_mode;
    commit_config.wfst_max_latency_ms = 150;
    let commit_refiner = load_transliterator(data_path, commit_config)?;
    let mut session = ImeSession::builder(live, HashMap::new())
        .visible_refiner(visible_refiner)
        .commit_refiner(commit_refiner)
        .options(ImeSessionOptions {
            segmented_preview: SegmentedPreviewMode::Enabled,
            ..Default::default()
        })
        .build();
    session.focus_in();
    for ch in roman.chars() {
        session.process_key_event(ch as u32, 0, 0);
    }
    let snapshot = session.snapshot();
    println!("raw_preedit: {}", snapshot.raw_preedit);
    println!("preedit:     {}", snapshot.preedit);
    println!("segmented_active: {}", snapshot.segmented_active);
    println!("focused_segment_index: {:?}", snapshot.focused_segment_index);
    println!("--- segment_preview ---");
    for (i, seg) in snapshot.segment_preview.iter().enumerate() {
        println!(
            "  [{i}] focused={} input={:?} output={:?}",
            seg.focused, seg.input, seg.output
        );
    }
    println!("--- top candidates for focused segment ---");
    for (i, cand) in snapshot.candidates.iter().take(10).enumerate() {
        println!("  [{i}] {}", cand);
    }
    for k in 1..snapshot.segment_preview.len() {
        session.process_key_event(0xFF53, 0, 0);
        let segment_snapshot = session.snapshot();
        println!(
            "--- candidates for segment {} (input={:?}) ---",
            k,
            segment_snapshot
                .segment_preview
                .get(k)
                .map(|entry| entry.input.clone())
                .unwrap_or_default()
        );
        for (i, cand) in segment_snapshot.candidates.iter().take(15).enumerate() {
            println!("  [{i}] {}", cand);
        }
    }
    Ok(())
}

fn run_shadow_eval(
    transliterator: &Transliterator,
    path: &str,
    emit_shadow_rows: bool,
    output_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let corpus_label = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path);
    let history = HashMap::new();
    let mut summary = ShadowSummary::default();
    let mut observations = Vec::new();

    for line in source.lines() {
        let query = line.trim();
        if query.is_empty() || query.starts_with('#') {
            continue;
        }

        let observation = transliterator.shadow_observation(query, &history);
        summary.record(&observation);
        observations.push(observation);
    }

    let rendered = render_shadow_eval_output(corpus_label, &observations, &summary, emit_shadow_rows);
    if let Some(path) = output_path {
        fs::write(path, &rendered)?;
    } else {
        print!("{}", rendered);
    }
    Ok(())
}

fn render_shadow_eval_output(
    corpus_label: &str,
    observations: &[ShadowObservation],
    summary: &ShadowSummary,
    emit_shadow_rows: bool,
) -> String {
    let mut output = String::new();
    if emit_shadow_rows {
        let _ = writeln!(&mut output, "{}", ShadowObservation::tsv_header());
        for observation in observations {
            let _ = writeln!(&mut output, "{}", observation.to_tsv_row());
        }
        let _ = writeln!(&mut output);
    }
    let _ = writeln!(&mut output, "report.corpus={}", corpus_label);
    output.push_str(&summary.format_report());
    output
}

#[cfg(test)]
mod tests {
    use roman_lookup::{ShadowMismatch, ShadowSummary};

    use super::render_shadow_eval_output;

    #[test]
    fn renders_rows_then_summary() {
        let observation = roman_lookup::ShadowObservation {
            mode: roman_lookup::DecoderMode::Shadow,
            input: "jea".to_owned(),
            mismatch: ShadowMismatch::Top1Match,
            composer_chunks: vec!["jea".to_owned()],
            composer_hint_chunks: Vec::new(),
            composer_pending_tail: String::new(),
            composer_fully_segmented: true,
            wfst_used_hint_chunks: false,
            wfst_top_segment_details: vec![roman_lookup::DecodeSegment {
                input: "jea".to_owned(),
                output: "ជា".to_owned(),
                weight_bps: 9_800,
            }],
            wfst_top_segments: vec!["jea=>ជា".to_owned()],
            legacy_latency_us: 10,
            wfst_latency_us: Some(8),
            legacy_failure: None,
            wfst_failure: None,
            legacy_top: Some("ជា".to_owned()),
            wfst_top: Some("ជា".to_owned()),
            legacy_top5: vec!["ជា".to_owned()],
            wfst_top5: vec!["ជា".to_owned()],
            legacy_top_in_wfst: true,
            wfst_top_in_legacy: true,
        };
        let mut summary = ShadowSummary::default();
        summary.record(&observation);

        let rendered = render_shadow_eval_output("shadow_eval_queries_v1.txt", &[observation], &summary, true);

        assert!(rendered.starts_with(roman_lookup::ShadowObservation::tsv_header()));
        assert!(rendered.contains("Shadow\tjea\ttop1_match"));
        assert!(rendered.contains("\nreport.corpus=shadow_eval_queries_v1.txt\n"));
        assert!(rendered.contains("\nsummary.total=1\n"));
    }
}
