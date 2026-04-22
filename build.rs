use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const DEFAULT_TSV_PATH: &str = "data/roman_lookup.tsv";
const DEFAULT_CSV_PATH: &str = "data/roman_lookup.csv";
const KHPOS_TRAIN_PATH: &str = "data/khPOS/corpus-draft-ver-1.0/data/after-replace/train.all";
const KHPOS_TAG_PATH: &str = "data/khPOS/corpus-draft-ver-1.0/data/after-replace/train.all.tag";
const MAGIC: &[u8; 4] = b"RLX1";
const KHPOS_MAGIC: &[u8; 4] = b"KPS1";
const MAX_JOINED_SURFACE_TOKENS: usize = 4;

#[derive(Clone, Copy)]
enum LexiconSourceFormat {
    Csv,
    Tsv,
}

fn main() {
    println!("cargo:rerun-if-changed={DEFAULT_CSV_PATH}");
    println!("cargo:rerun-if-changed={DEFAULT_TSV_PATH}");
    println!("cargo:rerun-if-changed={KHPOS_TRAIN_PATH}");
    println!("cargo:rerun-if-changed={KHPOS_TAG_PATH}");

    let (source, source_format) = match fs::read_to_string(DEFAULT_CSV_PATH) {
        Ok(source) => (source, LexiconSourceFormat::Csv),
        Err(_) => (
            fs::read_to_string(DEFAULT_TSV_PATH).expect("default lexicon CSV/TSV must be readable"),
            LexiconSourceFormat::Tsv,
        ),
    };
    let compiled = compile_lexicon(&source, source_format).expect("default lexicon CSV/TSV must compile");
    let khpos_train = fs::read_to_string(KHPOS_TRAIN_PATH).expect("khPOS after-replace train corpus must be readable");
    let khpos_tags = fs::read_to_string(KHPOS_TAG_PATH).expect("khPOS after-replace tag corpus must be readable");
    let compiled_khpos =
        compile_khpos_stats(&khpos_train, &khpos_tags).expect("khPOS after-replace corpus must compile");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let output_path = out_dir.join("roman_lookup.lexicon.bin");
    fs::write(&output_path, compiled).expect("compiled lexicon must be written");
    let khpos_output_path = out_dir.join("khpos.stats.bin");
    fs::write(&khpos_output_path, compiled_khpos).expect("compiled khPOS stats must be written");

    // When building for wasm32 with the fetch-data feature, copy the compiled
    // binary blobs into assets/data/ so Dioxus serves them as static files.
    let target = env::var("TARGET").unwrap_or_default();
    let fetch_data = env::var("CARGO_FEATURE_FETCH_DATA").is_ok();
    if target.starts_with("wasm32") && fetch_data {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
        let assets_data = PathBuf::from(manifest_dir).join("assets/data");
        fs::create_dir_all(&assets_data).expect("assets/data dir must be creatable");
        fs::copy(&output_path, assets_data.join("roman_lookup.lexicon.bin"))
            .expect("lexicon bin must copy to assets/data");
        fs::copy(&khpos_output_path, assets_data.join("khpos.stats.bin")).expect("khpos bin must copy to assets/data");
    }
}

fn compile_lexicon(source: &str, source_format: LexiconSourceFormat) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(source.len() + 8);
    output.extend_from_slice(MAGIC);

    let entries = parse_lexicon_entries(source, source_format)?;
    let entry_count = entries.len() as u32;
    output.extend_from_slice(&entry_count.to_le_bytes());

    for (line_no, (roman, target)) in entries.into_iter().enumerate() {
        if roman.contains('\0') || target.contains('\0') {
            return Err(format!("NUL byte is not supported on line {}", line_no + 1));
        }
        output.extend_from_slice(roman.as_bytes());
        output.push(0);
        output.extend_from_slice(target.as_bytes());
        output.push(0);
    }

    Ok(output)
}

fn parse_lexicon_entries(source: &str, source_format: LexiconSourceFormat) -> Result<Vec<(String, String)>, String> {
    match source_format {
        LexiconSourceFormat::Csv => parse_csv_entries(source),
        LexiconSourceFormat::Tsv => parse_tsv_entries(source),
    }
}

fn parse_tsv_entries(source: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries = Vec::new();
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let Some((roman, target)) = line.split_once('\t') else {
            return Err(format!("invalid TSV data format on line {}", line_no + 1));
        };
        entries.push((roman.to_owned(), target.to_owned()));
    }
    Ok(entries)
}

fn parse_csv_entries(source: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries = Vec::new();
    let mut first_row = true;
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = parse_csv_fields(line, line_no + 1)?;
        if fields.len() != 2 {
            return Err(format!(
                "invalid CSV data format on line {}: expected 2 columns, got {}",
                line_no + 1,
                fields.len()
            ));
        }
        if line_no == 0 {
            fields[0] = fields[0].trim_start_matches('\u{feff}').to_owned();
        }
        if first_row
            && fields[0].trim().eq_ignore_ascii_case("roman")
            && fields[1].trim().eq_ignore_ascii_case("target")
        {
            first_row = false;
            continue;
        }
        first_row = false;
        entries.push((fields.remove(0), fields.remove(0)));
    }
    Ok(entries)
}

fn parse_csv_fields(line: &str, line_no: usize) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '"' => {
                if current.is_empty() {
                    in_quotes = true;
                } else {
                    return Err(format!("invalid CSV data format on line {}: unexpected quote", line_no));
                }
            }
            ',' => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }

    if in_quotes {
        return Err(format!(
            "invalid CSV data format on line {}: unterminated quote",
            line_no
        ));
    }

    fields.push(current);
    Ok(fields)
}

fn compile_khpos_stats(train_source: &str, tag_source: &str) -> Result<Vec<u8>, String> {
    let train_lines = train_source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let tag_lines = tag_source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if train_lines.len() != tag_lines.len() {
        return Err(format!(
            "khPOS sentence count mismatch: {} train lines vs {} tag lines",
            train_lines.len(),
            tag_lines.len()
        ));
    }

    let mut word_unigrams = HashMap::<String, u32>::new();
    let mut word_bigrams = HashMap::<(String, String), u32>::new();
    let mut surface_unigrams = HashMap::<String, u32>::new();
    let mut tag_unigrams = HashMap::<String, u32>::new();
    let mut tag_bigrams = HashMap::<(String, String), u32>::new();
    let mut word_tag_counts = HashMap::<String, HashMap<String, u32>>::new();

    for (line_no, (train_line, tag_line)) in train_lines.iter().zip(tag_lines.iter()).enumerate() {
        let tagged_tokens = train_line.split_whitespace().collect::<Vec<_>>();
        let tags = tag_line.split_whitespace().collect::<Vec<_>>();
        if tagged_tokens.len() != tags.len() {
            return Err(format!(
                "khPOS token/tag mismatch on line {}: {} tagged tokens vs {} tags",
                line_no + 1,
                tagged_tokens.len(),
                tags.len()
            ));
        }

        let mut words = Vec::<String>::with_capacity(tagged_tokens.len());
        for (column, (tagged, expected_tag)) in tagged_tokens.iter().zip(tags.iter()).enumerate() {
            let Some((word, observed_tag)) = tagged.rsplit_once('/') else {
                return Err(format!(
                    "khPOS token missing word/tag separator on line {}, column {}",
                    line_no + 1,
                    column + 1
                ));
            };
            if word.is_empty() || observed_tag.is_empty() {
                return Err(format!(
                    "khPOS token has empty word/tag part on line {}, column {}",
                    line_no + 1,
                    column + 1
                ));
            }
            if observed_tag != *expected_tag {
                return Err(format!(
                    "khPOS tag mismatch on line {}, column {}: train token has '{}' but tag file has '{}'",
                    line_no + 1,
                    column + 1,
                    observed_tag,
                    expected_tag
                ));
            }

            let word = word.to_owned();
            let tag = (*expected_tag).to_owned();
            *word_unigrams.entry(word.clone()).or_default() += 1;
            *tag_unigrams.entry(tag.clone()).or_default() += 1;
            *word_tag_counts.entry(word.clone()).or_default().entry(tag).or_default() += 1;
            words.push(word);
        }

        for pair in words.windows(2) {
            *word_bigrams.entry((pair[0].clone(), pair[1].clone())).or_default() += 1;
        }
        for start in 0..words.len() {
            let mut joined = String::new();
            for token in words.iter().skip(start).take(MAX_JOINED_SURFACE_TOKENS) {
                joined.push_str(token);
                *surface_unigrams.entry(joined.clone()).or_default() += 1;
            }
        }
        for pair in tags.windows(2) {
            *tag_bigrams.entry((pair[0].to_owned(), pair[1].to_owned())).or_default() += 1;
        }
    }

    let surface_min_count = env::var("KHPOS_SURFACE_MIN_COUNT")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    if surface_min_count > 1 {
        surface_unigrams.retain(|_, count| *count >= surface_min_count);
    }

    if let Some(limit) = env::var("KHPOS_SURFACE_TOP_N")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|limit| *limit > 0)
    {
        trim_map_to_top_n(&mut surface_unigrams, limit);
    }

    let mut dominant_tags = word_tag_counts
        .into_iter()
        .map(|(word, tags)| {
            let mut tags = tags.into_iter().collect::<Vec<_>>();
            tags.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            let (tag, count) = tags.into_iter().next().expect("dominant tag source must be non-empty");
            (word, tag, count)
        })
        .collect::<Vec<_>>();
    dominant_tags.sort_by(|left, right| left.0.cmp(&right.0));

    let mut output = Vec::new();
    output.extend_from_slice(KHPOS_MAGIC);
    write_string_count_map(&mut output, &word_unigrams)?;
    write_pair_count_map(&mut output, &word_bigrams)?;
    write_string_count_map(&mut output, &surface_unigrams)?;
    write_string_count_map(&mut output, &tag_unigrams)?;
    write_pair_count_map(&mut output, &tag_bigrams)?;
    write_dominant_tags(&mut output, &dominant_tags)?;
    Ok(output)
}

fn trim_map_to_top_n(map: &mut HashMap<String, u32>, limit: usize) {
    if map.len() <= limit {
        return;
    }
    let mut ranked = map
        .iter()
        .map(|(token, count)| (token.clone(), *count))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let keep = ranked
        .into_iter()
        .take(limit)
        .map(|(token, _)| token)
        .collect::<std::collections::HashSet<_>>();
    map.retain(|token, _| keep.contains(token));
}

fn write_string_count_map(output: &mut Vec<u8>, map: &HashMap<String, u32>) -> Result<(), String> {
    let mut records = map.iter().collect::<Vec<_>>();
    records.sort_by(|left, right| left.0.cmp(right.0));
    write_u32(output, records.len() as u32);
    for (text, count) in records {
        write_string(output, text)?;
        write_u32(output, *count);
    }
    Ok(())
}

fn write_pair_count_map(output: &mut Vec<u8>, map: &HashMap<(String, String), u32>) -> Result<(), String> {
    let mut records = map.iter().collect::<Vec<_>>();
    records.sort_by(|left, right| left.0.cmp(right.0));
    write_u32(output, records.len() as u32);
    for ((left, right), count) in records {
        write_string(output, left)?;
        write_string(output, right)?;
        write_u32(output, *count);
    }
    Ok(())
}

fn write_dominant_tags(output: &mut Vec<u8>, records: &[(String, String, u32)]) -> Result<(), String> {
    write_u32(output, records.len() as u32);
    for (word, tag, count) in records {
        write_string(output, word)?;
        write_string(output, tag)?;
        write_u32(output, *count);
    }
    Ok(())
}

fn write_string(output: &mut Vec<u8>, text: &str) -> Result<(), String> {
    if text.contains('\0') {
        return Err(format!("khPOS compiled data does not support NUL bytes in '{}'", text));
    }
    output.extend_from_slice(text.as_bytes());
    output.push(0);
    Ok(())
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}
