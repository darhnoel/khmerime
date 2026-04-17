use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const DEFAULT_TSV_PATH: &str = "data/roman_lookup.tsv";
const KHPOS_TRAIN_PATH: &str = "data/khPOS/corpus-draft-ver-1.0/data/after-replace/train.all";
const KHPOS_TAG_PATH: &str = "data/khPOS/corpus-draft-ver-1.0/data/after-replace/train.all.tag";
const MAGIC: &[u8; 4] = b"RLX1";
const KHPOS_MAGIC: &[u8; 4] = b"KPS1";
const MAX_JOINED_SURFACE_TOKENS: usize = 4;

fn main() {
    println!("cargo:rerun-if-changed={DEFAULT_TSV_PATH}");
    println!("cargo:rerun-if-changed={KHPOS_TRAIN_PATH}");
    println!("cargo:rerun-if-changed={KHPOS_TAG_PATH}");

    let source = fs::read_to_string(DEFAULT_TSV_PATH).expect("default lexicon TSV must be readable");
    let compiled = compile_lexicon(&source).expect("default lexicon TSV must compile");
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
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let fetch_data = env::var("CARGO_FEATURE_FETCH_DATA").is_ok();
    if target_arch == "wasm32" && fetch_data {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
        let assets_data = PathBuf::from(manifest_dir).join("assets/data");
        fs::create_dir_all(&assets_data).expect("assets/data dir must be creatable");
        fs::copy(&output_path, assets_data.join("roman_lookup.lexicon.bin"))
            .expect("lexicon bin must copy to assets/data");
        fs::copy(&khpos_output_path, assets_data.join("khpos.stats.bin")).expect("khpos bin must copy to assets/data");
    }
}

fn compile_lexicon(source: &str) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(source.len() + 8);
    output.extend_from_slice(MAGIC);

    let entry_count = source.lines().filter(|line| !line.is_empty()).count() as u32;
    output.extend_from_slice(&entry_count.to_le_bytes());

    for (line_no, line) in source.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let Some((roman, target)) = line.split_once('\t') else {
            return Err(format!("invalid data format on line {}", line_no + 1));
        };
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
