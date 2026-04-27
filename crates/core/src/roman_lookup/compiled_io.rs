use super::*;

// Compiled data is intentionally simple and append-only: fixed magic bytes,
// counts, then length-prefixed UTF-8 records and little-endian integers. Keep
// parsers strict so stale generated blobs fail during startup instead of
// producing corrupted candidates.

impl CorpusStats {
    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub(super) fn from_default_data() -> Result<Self> {
        parse_compiled_khpos_stats(DEFAULT_COMPILED_KHPOS_STATS)
    }

    pub(super) fn dominant_tag(&self, word: &str) -> Option<&str> {
        self.dominant_word_tags.get(word).map(|entry| entry.tag.as_str())
    }
}

impl NextWordStats {
    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    pub(super) fn from_default_data() -> Result<Self> {
        parse_compiled_next_word_stats(DEFAULT_COMPILED_NEXT_WORD_STATS)
    }
}

#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
pub(super) fn parse_tsv(source: &str) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let Some((roman, target)) = line.split_once('\t') else {
            return Err(LexiconError::Parse(format!(
                "invalid TSV data format on line {}",
                line_no + 1
            )));
        };
        entries.push(Entry {
            roman: roman.to_owned(),
            target: target.to_owned(),
        });
    }
    Ok(entries)
}

#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
pub(super) fn parse_csv(source: &str) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let mut first_row = true;
    for (line_no, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = parse_csv_fields(line, line_no + 1)?;
        if fields.len() != 2 {
            return Err(LexiconError::Parse(format!(
                "invalid CSV data format on line {}: expected 2 columns, got {}",
                line_no + 1,
                fields.len()
            )));
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
        entries.push(Entry {
            roman: fields.remove(0),
            target: fields.remove(0),
        });
    }
    Ok(entries)
}

#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
fn parse_csv_fields(line: &str, line_no: usize) -> Result<Vec<String>> {
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
                    return Err(LexiconError::Parse(format!(
                        "invalid CSV data format on line {}: unexpected quote",
                        line_no
                    )));
                }
            }
            ',' => fields.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }

    if in_quotes {
        return Err(LexiconError::Parse(format!(
            "invalid CSV data format on line {}: unterminated quote",
            line_no
        )));
    }

    fields.push(current);
    Ok(fields)
}

pub(super) fn parse_compiled_lexicon(source: &[u8]) -> Result<Vec<Entry>> {
    if source.len() < 8 || &source[..4] != COMPILED_MAGIC {
        return Err(LexiconError::Parse("invalid compiled lexicon header".to_owned()));
    }

    let entry_count = u32::from_le_bytes(source[4..8].try_into().expect("header slice has fixed width")) as usize;
    let mut offset = 8usize;
    let mut entries = Vec::with_capacity(entry_count);
    while offset < source.len() {
        let roman_end = find_nul(source, offset)
            .ok_or_else(|| LexiconError::Parse("invalid compiled lexicon payload".to_owned()))?;
        let target_start = roman_end + 1;
        let target_end = find_nul(source, target_start)
            .ok_or_else(|| LexiconError::Parse("invalid compiled lexicon payload".to_owned()))?;
        let roman = std::str::from_utf8(&source[offset..roman_end])
            .map_err(|_| LexiconError::Parse("compiled lexicon contains invalid UTF-8".to_owned()))?;
        let target = std::str::from_utf8(&source[target_start..target_end])
            .map_err(|_| LexiconError::Parse("compiled lexicon contains invalid UTF-8".to_owned()))?;
        entries.push(Entry {
            roman: roman.to_owned(),
            target: target.to_owned(),
        });
        offset = target_end + 1;
    }

    if entries.len() != entry_count {
        return Err(LexiconError::Parse("compiled lexicon entry count mismatch".to_owned()));
    }

    Ok(entries)
}

pub(super) fn parse_compiled_khpos_stats(source: &[u8]) -> Result<CorpusStats> {
    if source.len() < 4 || &source[..4] != KHPOS_MAGIC {
        return Err(LexiconError::Parse("invalid compiled khPOS stats header".to_owned()));
    }

    let mut offset = 4usize;
    let word_unigrams = read_string_count_map(source, &mut offset)?;
    let word_bigrams = read_pair_count_map(source, &mut offset)?;
    let surface_unigrams = read_string_count_map(source, &mut offset)?;
    let tag_unigrams = read_string_count_map(source, &mut offset)?;
    let tag_bigrams = read_pair_count_map(source, &mut offset)?;
    let dominant_word_tags = read_dominant_tags(source, &mut offset)?;
    if offset != source.len() {
        return Err(LexiconError::Parse(
            "compiled khPOS stats has trailing bytes".to_owned(),
        ));
    }

    Ok(CorpusStats {
        word_unigrams,
        word_bigrams,
        surface_unigrams,
        tag_unigrams,
        tag_bigrams,
        dominant_word_tags,
    })
}

pub(super) fn parse_compiled_next_word_stats(source: &[u8]) -> Result<NextWordStats> {
    if source.len() < 4 || &source[..4] != NEXT_WORD_MAGIC {
        return Err(LexiconError::Parse(
            "invalid compiled next-word stats header".to_owned(),
        ));
    }

    let mut offset = 4usize;
    let unigrams = read_string_count_map(source, &mut offset)?;
    let bigram_pairs = read_pair_count_map(source, &mut offset)?;
    if offset != source.len() {
        return Err(LexiconError::Parse(
            "compiled next-word stats has trailing bytes".to_owned(),
        ));
    }

    let mut bigrams = HashMap::<String, Vec<(String, u32)>>::new();
    for ((left, right), count) in bigram_pairs {
        bigrams.entry(left).or_default().push((right, count));
    }
    for rows in bigrams.values_mut() {
        rows.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        rows.dedup_by(|left, right| left.0 == right.0);
    }

    let mut ranked_unigrams = unigrams
        .iter()
        .map(|(word, count)| (word.clone(), *count))
        .collect::<Vec<_>>();
    ranked_unigrams.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    Ok(NextWordStats {
        unigrams,
        bigrams,
        ranked_unigrams,
    })
}

fn find_nul(source: &[u8], start: usize) -> Option<usize> {
    source[start..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|relative| start + relative)
}

fn read_string_count_map(source: &[u8], offset: &mut usize) -> Result<HashMap<String, u32>> {
    let count = read_u32(source, offset)? as usize;
    let mut output = HashMap::with_capacity(count);
    for _ in 0..count {
        let text = read_nul_terminated_str(source, offset)?;
        let value = read_u32(source, offset)?;
        output.insert(text, value);
    }
    Ok(output)
}

fn read_pair_count_map(source: &[u8], offset: &mut usize) -> Result<HashMap<(String, String), u32>> {
    let count = read_u32(source, offset)? as usize;
    let mut output = HashMap::with_capacity(count);
    for _ in 0..count {
        let left = read_nul_terminated_str(source, offset)?;
        let right = read_nul_terminated_str(source, offset)?;
        let value = read_u32(source, offset)?;
        output.insert((left, right), value);
    }
    Ok(output)
}

fn read_dominant_tags(source: &[u8], offset: &mut usize) -> Result<HashMap<String, DominantTag>> {
    let count = read_u32(source, offset)? as usize;
    let mut output = HashMap::with_capacity(count);
    for _ in 0..count {
        let word = read_nul_terminated_str(source, offset)?;
        let tag = read_nul_terminated_str(source, offset)?;
        let support = read_u32(source, offset)?;
        output.insert(word, DominantTag { tag, support });
    }
    Ok(output)
}

fn read_u32(source: &[u8], offset: &mut usize) -> Result<u32> {
    if source.len().saturating_sub(*offset) < 4 {
        return Err(LexiconError::Parse(
            "compiled khPOS stats payload is truncated".to_owned(),
        ));
    }
    let value = u32::from_le_bytes(source[*offset..*offset + 4].try_into().expect("slice length checked"));
    *offset += 4;
    Ok(value)
}

fn read_nul_terminated_str(source: &[u8], offset: &mut usize) -> Result<String> {
    let end = find_nul(source, *offset)
        .ok_or_else(|| LexiconError::Parse("compiled khPOS stats string is missing NUL terminator".to_owned()))?;
    let value = std::str::from_utf8(&source[*offset..end])
        .map_err(|_| LexiconError::Parse("compiled khPOS stats contains invalid UTF-8".to_owned()))?
        .to_owned();
    *offset = end + 1;
    Ok(value)
}
