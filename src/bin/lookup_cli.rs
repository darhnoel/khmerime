use std::collections::HashMap;
use std::env;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::process;

use roman_lookup::{DecoderConfig, DecoderMode, ShadowObservation, ShadowSummary, Transliterator};

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

    let transliterator = if let Some(path) = data_path {
        Transliterator::from_tsv_path_with_config(path, config.clone())?
    } else {
        Transliterator::from_default_data_with_config(config.clone())?
    };

    let Some(command) = args.get(index) else {
        print_usage(&args[0]);
        process::exit(2);
    };
    index += 1;

    match command.as_str() {
        "stats" => {
            println!("entries: {}", transliterator.entries().len());
        }
        "suggest" => {
            let Some(query) = args.get(index) else {
                print_usage(&args[0]);
                process::exit(2);
            };
            let history = HashMap::new();
            for suggestion in transliterator.suggest(query, &history) {
                println!("{}", suggestion);
            }
        }
        "shadow-eval" => {
            let Some(path) = args.get(index) else {
                print_usage(&args[0]);
                process::exit(2);
            };
            run_shadow_eval(&transliterator, path, emit_shadow_rows, output_path.as_deref())?;
        }
        _ => {
            print_usage(&args[0]);
            process::exit(2);
        }
    }

    Ok(())
}

fn print_usage(bin: &str) {
    eprintln!("Usage:");
    eprintln!(
        "  {} [--data <path/to/data.tsv>] [--decoder-mode legacy|shadow|wfst|hybrid] [--shadow-log] [--shadow-sample-bps 0..10000] stats",
        bin
    );
    eprintln!(
        "  {} [--data <path/to/data.tsv>] [--decoder-mode legacy|shadow|wfst|hybrid] [--shadow-log] [--shadow-sample-bps 0..10000] suggest <roman>",
        bin
    );
    eprintln!(
        "  {} [--data <path/to/data.tsv>] [--decoder-mode legacy|shadow|wfst|hybrid] [--emit-shadow-rows] [--output <report.txt>] shadow-eval <queries.txt>",
        bin
    );
}

fn parse_decoder_mode(value: &str) -> Option<DecoderMode> {
    match value {
        "legacy" => Some(DecoderMode::Legacy),
        "shadow" => Some(DecoderMode::Shadow),
        "wfst" => Some(DecoderMode::Wfst),
        "hybrid" => Some(DecoderMode::Hybrid),
        _ => None,
    }
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
