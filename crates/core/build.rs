use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DATA_PATHS_CONFIG_PATH: &str = "../../config/data_paths.toml";
const DEFAULT_TSV_PATH: &str = "../../data/roman_lookup.tsv";
const DEFAULT_CSV_PATH: &str = "../../data/roman_lookup.csv";
const DEFAULT_KHPOS_TRAIN_PATH: &str = "../../data/khPOS/corpus-draft-ver-1.0/data/after-replace/train.all";
const DEFAULT_KHPOS_TAG_PATH: &str = "../../data/khPOS/corpus-draft-ver-1.0/data/after-replace/train.all.tag";
const DEFAULT_MOBILE_KEYBOARD_1GRAM_PATH: &str =
    "../../data/khmerlang-mobile-keyboard-data/keyboard-data/extracted/mobile-keyboard-data-1gram.csv";
const DEFAULT_MOBILE_KEYBOARD_2GRAM_PATH: &str =
    "../../data/khmerlang-mobile-keyboard-data/keyboard-data/extracted/mobile-keyboard-data-2gram.csv";
const MAGIC: &[u8; 4] = b"RLX1";
const KHPOS_MAGIC: &[u8; 4] = b"KPS1";
const NEXT_WORD_MAGIC: &[u8; 4] = b"NWS1";
const MAX_JOINED_SURFACE_TOKENS: usize = 4;

#[derive(Clone, Copy)]
enum LexiconSourceFormat {
    Csv,
    Tsv,
}

#[derive(Clone, Debug)]
struct BuildDataPaths {
    lexicon_csv: String,
    lexicon_tsv: String,
    khpos_train: String,
    khpos_tag: String,
    mobile_keyboard_1gram: String,
    mobile_keyboard_2gram: String,
}

impl Default for BuildDataPaths {
    fn default() -> Self {
        Self {
            lexicon_csv: DEFAULT_CSV_PATH.to_owned(),
            lexicon_tsv: DEFAULT_TSV_PATH.to_owned(),
            khpos_train: DEFAULT_KHPOS_TRAIN_PATH.to_owned(),
            khpos_tag: DEFAULT_KHPOS_TAG_PATH.to_owned(),
            mobile_keyboard_1gram: DEFAULT_MOBILE_KEYBOARD_1GRAM_PATH.to_owned(),
            mobile_keyboard_2gram: DEFAULT_MOBILE_KEYBOARD_2GRAM_PATH.to_owned(),
        }
    }
}

fn normalize_workspace_path(path: &str) -> String {
    if Path::new(path).is_absolute() || Path::new(path).exists() {
        return path.to_owned();
    }
    let candidate = format!("../../{path}");
    if Path::new(&candidate).exists() {
        candidate
    } else {
        path.to_owned()
    }
}

fn main() {
    let mut data_paths = load_data_paths_from_config();
    data_paths.lexicon_csv = normalize_workspace_path(&data_paths.lexicon_csv);
    data_paths.lexicon_tsv = normalize_workspace_path(&data_paths.lexicon_tsv);
    data_paths.khpos_train = normalize_workspace_path(&data_paths.khpos_train);
    data_paths.khpos_tag = normalize_workspace_path(&data_paths.khpos_tag);
    data_paths.mobile_keyboard_1gram = normalize_workspace_path(&data_paths.mobile_keyboard_1gram);
    data_paths.mobile_keyboard_2gram = normalize_workspace_path(&data_paths.mobile_keyboard_2gram);
    println!("cargo:rerun-if-changed={}", data_paths.lexicon_csv);
    println!("cargo:rerun-if-changed={}", data_paths.lexicon_tsv);
    println!("cargo:rerun-if-changed={}", data_paths.khpos_train);
    println!("cargo:rerun-if-changed={}", data_paths.khpos_tag);
    println!("cargo:rerun-if-changed={}", data_paths.mobile_keyboard_1gram);
    println!("cargo:rerun-if-changed={}", data_paths.mobile_keyboard_2gram);

    let (source, source_format) = match fs::read_to_string(&data_paths.lexicon_csv) {
        Ok(source) => (source, LexiconSourceFormat::Csv),
        Err(_) => (
            fs::read_to_string(&data_paths.lexicon_tsv).expect("default lexicon CSV/TSV must be readable"),
            LexiconSourceFormat::Tsv,
        ),
    };
    let compiled = compile_lexicon(&source, source_format).expect("default lexicon CSV/TSV must compile");
    let khpos_train =
        fs::read_to_string(&data_paths.khpos_train).expect("khPOS after-replace train corpus must be readable");
    let khpos_tags =
        fs::read_to_string(&data_paths.khpos_tag).expect("khPOS after-replace tag corpus must be readable");
    let compiled_khpos =
        compile_khpos_stats(&khpos_train, &khpos_tags).expect("khPOS after-replace corpus must compile");
    let mobile_keyboard_1gram =
        fs::read_to_string(&data_paths.mobile_keyboard_1gram).expect("mobile keyboard 1-gram data must be readable");
    let mobile_keyboard_2gram =
        fs::read_to_string(&data_paths.mobile_keyboard_2gram).expect("mobile keyboard 2-gram data must be readable");
    let compiled_next_word = compile_next_word_stats(&mobile_keyboard_1gram, &mobile_keyboard_2gram)
        .expect("mobile keyboard n-gram data must compile");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let output_path = out_dir.join("roman_lookup.lexicon.bin");
    fs::write(&output_path, compiled).expect("compiled lexicon must be written");
    let khpos_output_path = out_dir.join("khpos.stats.bin");
    fs::write(&khpos_output_path, compiled_khpos).expect("compiled khPOS stats must be written");
    let next_word_output_path = out_dir.join("next_word.stats.bin");
    fs::write(&next_word_output_path, compiled_next_word).expect("compiled next-word stats must be written");

    // When building for wasm32 with the fetch-data feature, copy the compiled
    // binary blobs into assets/data/ so Dioxus serves them as static files.
    let target = env::var("TARGET").unwrap_or_default();
    let fetch_data = env::var("CARGO_FEATURE_FETCH_DATA").is_ok();
    if target.starts_with("wasm32") && fetch_data {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
        let assets_data = PathBuf::from(manifest_dir).join("../../assets/data");
        fs::create_dir_all(&assets_data).expect("assets/data dir must be creatable");
        fs::copy(&output_path, assets_data.join("roman_lookup.lexicon.bin"))
            .expect("lexicon bin must copy to assets/data");
        fs::copy(&khpos_output_path, assets_data.join("khpos.stats.bin")).expect("khpos bin must copy to assets/data");
        fs::copy(&next_word_output_path, assets_data.join("next_word.stats.bin"))
            .expect("next-word bin must copy to assets/data");
    }
}

fn load_data_paths_from_config() -> BuildDataPaths {
    println!("cargo:rerun-if-changed={DATA_PATHS_CONFIG_PATH}");
    let mut paths = BuildDataPaths::default();
    let Ok(source) = fs::read_to_string(DATA_PATHS_CONFIG_PATH) else {
        return paths;
    };
    if let Err(error) = apply_data_paths_config(&source, &mut paths) {
        panic!("{DATA_PATHS_CONFIG_PATH} parse failed: {error}");
    }
    paths
}

fn apply_data_paths_config(source: &str, paths: &mut BuildDataPaths) -> Result<(), String> {
    let mut in_data_paths_section = false;
    for (line_no, raw_line) in source.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let section = line[1..line.len() - 1].trim();
            in_data_paths_section = section == "data_paths";
            continue;
        }
        if !in_data_paths_section {
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!("invalid config format on line {}", line_no + 1));
        };
        let key = raw_key.trim();
        let value = parse_data_path_value(raw_value.trim(), line_no + 1)?;
        if value.is_empty() {
            return Err(format!("empty value for '{}' on line {}", key, line_no + 1));
        }
        match key {
            "lexicon_csv" => paths.lexicon_csv = value,
            "lexicon_tsv" => paths.lexicon_tsv = value,
            "khpos_train" => paths.khpos_train = value,
            "khpos_tag" => paths.khpos_tag = value,
            "mobile_keyboard_1gram" => paths.mobile_keyboard_1gram = value,
            "mobile_keyboard_2gram" => paths.mobile_keyboard_2gram = value,
            _ => return Err(format!("unknown key '{}' in [data_paths] on line {}", key, line_no + 1)),
        }
    }
    Ok(())
}

fn parse_data_path_value(raw: &str, line_no: usize) -> Result<String, String> {
    if raw.starts_with('"') {
        if !raw.ends_with('"') || raw.len() < 2 {
            return Err(format!("unterminated quoted value on line {}", line_no));
        }
        let content = &raw[1..raw.len() - 1];
        return Ok(content.replace("\\\\", "\\").replace("\\\"", "\""));
    }
    Ok(raw.to_owned())
}

fn compile_next_word_stats(unigram_source: &str, bigram_source: &str) -> Result<Vec<u8>, String> {
    let mut unigram_counts = HashMap::<String, u32>::new();
    let mut bigram_counts = HashMap::<(String, String), u32>::new();

    parse_unigram_rows(unigram_source, &mut unigram_counts)?;
    parse_bigram_rows(bigram_source, &mut bigram_counts)?;

    let mut output = Vec::new();
    output.extend_from_slice(NEXT_WORD_MAGIC);
    write_string_count_map(&mut output, &unigram_counts)?;
    write_pair_count_map(&mut output, &bigram_counts)?;
    Ok(output)
}

fn parse_unigram_rows(source: &str, unigram_counts: &mut HashMap<String, u32>) -> Result<(), String> {
    let mut first_row = true;
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = parse_csv_fields(line, line_no + 1)?;
        if fields.len() < 2 {
            continue;
        }
        if line_no == 0 {
            fields[0] = fields[0].trim_start_matches('\u{feff}').to_owned();
        }
        if first_row
            && fields[0].trim().eq_ignore_ascii_case("word")
            && fields[1].trim().eq_ignore_ascii_case("frequency")
        {
            first_row = false;
            continue;
        }
        first_row = false;

        let raw_word = fields[0].trim();
        let Ok(raw_frequency) = fields[1].trim().parse::<u32>() else {
            continue;
        };
        if raw_frequency == 0 {
            continue;
        }

        let word = normalize_next_word_token(raw_word);
        if !is_khmer_token(&word) {
            continue;
        }
        *unigram_counts.entry(word).or_default() += raw_frequency;
    }
    Ok(())
}

fn parse_bigram_rows(source: &str, bigram_counts: &mut HashMap<(String, String), u32>) -> Result<(), String> {
    let mut first_row = true;
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = parse_csv_fields(line, line_no + 1)?;
        if fields.len() < 2 {
            continue;
        }
        if line_no == 0 {
            fields[0] = fields[0].trim_start_matches('\u{feff}').to_owned();
        }
        if first_row
            && fields[0].trim().eq_ignore_ascii_case("word")
            && fields[1].trim().eq_ignore_ascii_case("frequency")
        {
            first_row = false;
            continue;
        }
        first_row = false;

        let raw_phrase = fields[0].trim();
        let Ok(raw_frequency) = fields[1].trim().parse::<u32>() else {
            continue;
        };
        if raw_frequency == 0 {
            continue;
        }

        let parts = raw_phrase.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 2 {
            continue;
        }
        let left = normalize_next_word_token(parts[0]);
        let right = normalize_next_word_token(parts[1]);
        if !is_valid_left_context(&left) || !is_khmer_token(&right) {
            continue;
        }
        *bigram_counts.entry((left, right)).or_default() += raw_frequency;
    }
    Ok(())
}

fn normalize_next_word_token(token: &str) -> String {
    token.trim().chars().filter(|ch| *ch != '\u{200b}').collect::<String>()
}

fn is_valid_left_context(token: &str) -> bool {
    matches!(token, "<s>" | "<num>" | "<oth>" | "<unk>") || is_khmer_token(token)
}

fn is_khmer_token(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|ch| {
            ('\u{1780}'..='\u{17ff}').contains(&ch)
                || ('\u{19e0}'..='\u{19ff}').contains(&ch)
                || ch == '\u{200c}'
                || ch == '\u{200d}'
        })
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
