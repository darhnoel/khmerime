use std::cmp::Ordering;
use std::ops::Range;

use super::dictionary_image_format::*;
use super::{LexiconError, Result};

#[derive(Clone, Copy, Debug)]
pub(crate) struct DictionaryImageView<'a> {
    string_refs: &'a [u8],
    string_data: &'a [u8],
    entries: &'a [u8],
    exact_keys: &'a [u8],
    exact_postings: &'a [u8],
    alias_keys: &'a [u8],
    alias_postings: &'a [u8],
    gram_keys: &'a [u8],
    gram_postings: &'a [u8],
    legacy_roman_keys: &'a [u8],
    legacy_roman_postings: &'a [u8],
    legacy_normalized_keys: &'a [u8],
    legacy_normalized_postings: &'a [u8],
    legacy_target_keys: &'a [u8],
    legacy_target_postings: &'a [u8],
    legacy_prefix_keys: &'a [u8],
    legacy_prefix_postings: &'a [u8],
    legacy_target_frequencies: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DictionaryEntryView<'a> {
    image: DictionaryImageView<'a>,
    offset: usize,
}

impl<'a> DictionaryImageView<'a> {
    pub(crate) fn parse(source: &'a [u8]) -> Result<Self> {
        if source.len() < HEADER_LEN || &source[..4] != DICTIONARY_IMAGE_MAGIC {
            return Err(LexiconError::Parse("invalid dictionary image header".to_owned()));
        }
        let schema_version = read_u32_at(source, 4)?;
        if schema_version != DICTIONARY_IMAGE_SCHEMA_VERSION {
            return Err(LexiconError::Parse(format!(
                "unsupported dictionary image schema version {schema_version}"
            )));
        }
        let section_count = read_u32_at(source, 8)? as usize;
        if section_count != DICTIONARY_IMAGE_SECTION_COUNT as usize {
            return Err(LexiconError::Parse(
                "dictionary image section count mismatch".to_owned(),
            ));
        }
        let table_len = section_count
            .checked_mul(SECTION_RECORD_LEN)
            .and_then(|len| HEADER_LEN.checked_add(len))
            .ok_or_else(|| LexiconError::Parse("dictionary image section table is too large".to_owned()))?;
        if source.len() < table_len {
            return Err(LexiconError::Parse(
                "dictionary image section table is truncated".to_owned(),
            ));
        }

        let view = Self {
            string_refs: section(source, section_count, SECTION_STRING_REFS)?,
            string_data: section(source, section_count, SECTION_STRING_DATA)?,
            entries: section(source, section_count, SECTION_ENTRIES)?,
            exact_keys: section(source, section_count, SECTION_EXACT_KEYS)?,
            exact_postings: section(source, section_count, SECTION_EXACT_POSTINGS)?,
            alias_keys: section(source, section_count, SECTION_ALIAS_KEYS)?,
            alias_postings: section(source, section_count, SECTION_ALIAS_POSTINGS)?,
            gram_keys: section(source, section_count, SECTION_GRAM_KEYS)?,
            gram_postings: section(source, section_count, SECTION_GRAM_POSTINGS)?,
            legacy_roman_keys: section(source, section_count, SECTION_LEGACY_ROMAN_KEYS)?,
            legacy_roman_postings: section(source, section_count, SECTION_LEGACY_ROMAN_POSTINGS)?,
            legacy_normalized_keys: section(source, section_count, SECTION_LEGACY_NORMALIZED_KEYS)?,
            legacy_normalized_postings: section(source, section_count, SECTION_LEGACY_NORMALIZED_POSTINGS)?,
            legacy_target_keys: section(source, section_count, SECTION_LEGACY_TARGET_KEYS)?,
            legacy_target_postings: section(source, section_count, SECTION_LEGACY_TARGET_POSTINGS)?,
            legacy_prefix_keys: section(source, section_count, SECTION_LEGACY_PREFIX_KEYS)?,
            legacy_prefix_postings: section(source, section_count, SECTION_LEGACY_PREFIX_POSTINGS)?,
            legacy_target_frequencies: section(source, section_count, SECTION_LEGACY_TARGET_FREQUENCIES)?,
        };
        view.validate_record_sections()?;
        Ok(view)
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.entries.len() / ENTRY_RECORD_LEN
    }

    pub(crate) fn exact_key_count(&self) -> usize {
        self.exact_keys.len() / KEY_RANGE_RECORD_LEN
    }

    pub(crate) fn alias_key_count(&self) -> usize {
        self.alias_keys.len() / KEY_RANGE_RECORD_LEN
    }

    pub(crate) fn gram_key_count(&self) -> usize {
        self.gram_keys.len() / KEY_RANGE_RECORD_LEN
    }

    pub(crate) fn legacy_roman_key_count(&self) -> usize {
        self.legacy_roman_keys.len() / KEY_RANGE_RECORD_LEN
    }

    pub(crate) fn legacy_normalized_key_count(&self) -> usize {
        self.legacy_normalized_keys.len() / KEY_RANGE_RECORD_LEN
    }

    pub(crate) fn legacy_target_key_count(&self) -> usize {
        self.legacy_target_keys.len() / KEY_RANGE_RECORD_LEN
    }

    pub(crate) fn legacy_prefix_key_count(&self) -> usize {
        self.legacy_prefix_keys.len() / KEY_RANGE_RECORD_LEN
    }

    pub(crate) fn entry(&self, entry_id: u32) -> Result<DictionaryEntryView<'a>> {
        let offset = (entry_id as usize)
            .checked_mul(ENTRY_RECORD_LEN)
            .ok_or_else(|| LexiconError::Parse("dictionary image entry id overflow".to_owned()))?;
        if self.entries.len().saturating_sub(offset) < ENTRY_RECORD_LEN {
            return Err(LexiconError::Parse("dictionary image entry id out of range".to_owned()));
        }
        Ok(DictionaryEntryView { image: *self, offset })
    }

    pub(crate) fn exact_postings(&self, key: &str) -> Result<Option<DictionaryPostings<'a>>> {
        self.lookup_key(self.exact_keys, self.exact_postings, key)
    }

    pub(crate) fn alias_postings(&self, key: &str) -> Result<Option<DictionaryPostings<'a>>> {
        self.lookup_key(self.alias_keys, self.alias_postings, key)
    }

    pub(crate) fn gram_postings(&self, key: &str) -> Result<Option<DictionaryPostings<'a>>> {
        self.lookup_key(self.gram_keys, self.gram_postings, key)
    }

    pub(crate) fn legacy_roman_targets(&self, roman: &str) -> Result<Option<Vec<&'a str>>> {
        self.lookup_string_values(self.legacy_roman_keys, self.legacy_roman_postings, roman)
    }

    pub(crate) fn legacy_normalized_targets(&self, normalized: &str) -> Result<Option<Vec<&'a str>>> {
        self.lookup_string_values(self.legacy_normalized_keys, self.legacy_normalized_postings, normalized)
    }

    pub(crate) fn legacy_target_romans(&self, target: &str) -> Result<Option<Vec<&'a str>>> {
        self.lookup_string_values(self.legacy_target_keys, self.legacy_target_postings, target)
    }

    pub(crate) fn legacy_prefix_romans(&self, prefix: &str) -> Result<Option<Vec<&'a str>>> {
        self.lookup_string_values(self.legacy_prefix_keys, self.legacy_prefix_postings, prefix)
    }

    pub(crate) fn legacy_all_romans(&self) -> Result<Vec<&'a str>> {
        self.key_strings(self.legacy_roman_keys)
    }

    pub(crate) fn legacy_target_frequency(&self, target: &str) -> Result<Option<u32>> {
        let mut low = 0usize;
        let mut high = self.legacy_target_frequencies.len() / STRING_U32_RECORD_LEN;
        while low < high {
            let mid = low + (high - low) / 2;
            let offset = mid * STRING_U32_RECORD_LEN;
            let target_id = read_u32_at(self.legacy_target_frequencies, offset)?;
            let key = self.string(target_id)?;
            match key.cmp(target) {
                Ordering::Less => low = mid + 1,
                Ordering::Equal => return read_u32_at(self.legacy_target_frequencies, offset + 4).map(Some),
                Ordering::Greater => high = mid,
            }
        }
        Ok(None)
    }

    fn lookup_string_values(&self, keys: &'a [u8], postings: &'a [u8], query: &str) -> Result<Option<Vec<&'a str>>> {
        let Some(ids) = self.lookup_key(keys, postings, query)? else {
            return Ok(None);
        };
        let mut values = Vec::new();
        for id in ids {
            values.push(self.string(id)?);
        }
        Ok(Some(values))
    }

    fn key_strings(&self, keys: &'a [u8]) -> Result<Vec<&'a str>> {
        let mut values = Vec::with_capacity(keys.len() / KEY_RANGE_RECORD_LEN);
        for offset in (0..keys.len()).step_by(KEY_RANGE_RECORD_LEN) {
            values.push(self.string(read_u32_at(keys, offset)?)?);
        }
        Ok(values)
    }

    fn lookup_key(&self, keys: &'a [u8], postings: &'a [u8], query: &str) -> Result<Option<DictionaryPostings<'a>>> {
        let mut low = 0usize;
        let mut high = keys.len() / KEY_RANGE_RECORD_LEN;
        while low < high {
            let mid = low + (high - low) / 2;
            let offset = mid * KEY_RANGE_RECORD_LEN;
            let key_id = read_u32_at(keys, offset)?;
            let key = self.string(key_id)?;
            match key.cmp(query) {
                Ordering::Less => low = mid + 1,
                Ordering::Equal => {
                    let start = read_u32_at(keys, offset + 4)? as usize;
                    let len = read_u32_at(keys, offset + 8)? as usize;
                    let byte_start = start
                        .checked_mul(4)
                        .ok_or_else(|| LexiconError::Parse("dictionary image posting range overflow".to_owned()))?;
                    let byte_len = len
                        .checked_mul(4)
                        .ok_or_else(|| LexiconError::Parse("dictionary image posting range overflow".to_owned()))?;
                    let range = checked_range(postings, byte_start, byte_len)?;
                    return Ok(Some(DictionaryPostings {
                        source: &postings[range],
                        index: 0,
                    }));
                }
                Ordering::Greater => high = mid,
            }
        }
        Ok(None)
    }

    fn string(&self, string_id: u32) -> Result<&'a str> {
        if string_id == MISSING_STRING_ID {
            return Err(LexiconError::Parse(
                "dictionary image missing-string sentinel used as required string".to_owned(),
            ));
        }
        let offset = (string_id as usize)
            .checked_mul(STRING_REF_RECORD_LEN)
            .ok_or_else(|| LexiconError::Parse("dictionary image string id overflow".to_owned()))?;
        if self.string_refs.len().saturating_sub(offset) < STRING_REF_RECORD_LEN {
            return Err(LexiconError::Parse(
                "dictionary image string id out of range".to_owned(),
            ));
        }
        let start = read_u32_at(self.string_refs, offset)? as usize;
        let len = read_u32_at(self.string_refs, offset + 4)? as usize;
        let range = checked_range(self.string_data, start, len)?;
        std::str::from_utf8(&self.string_data[range])
            .map_err(|_| LexiconError::Parse("dictionary image contains invalid UTF-8".to_owned()))
    }

    fn optional_string(&self, string_id: u32) -> Result<Option<&'a str>> {
        if string_id == MISSING_STRING_ID {
            Ok(None)
        } else {
            self.string(string_id).map(Some)
        }
    }

    fn validate_record_sections(&self) -> Result<()> {
        for (label, len, record_len) in [
            ("string refs", self.string_refs.len(), STRING_REF_RECORD_LEN),
            ("entries", self.entries.len(), ENTRY_RECORD_LEN),
            ("exact keys", self.exact_keys.len(), KEY_RANGE_RECORD_LEN),
            ("alias keys", self.alias_keys.len(), KEY_RANGE_RECORD_LEN),
            ("gram keys", self.gram_keys.len(), KEY_RANGE_RECORD_LEN),
            ("legacy roman keys", self.legacy_roman_keys.len(), KEY_RANGE_RECORD_LEN),
            (
                "legacy normalized keys",
                self.legacy_normalized_keys.len(),
                KEY_RANGE_RECORD_LEN,
            ),
            (
                "legacy target keys",
                self.legacy_target_keys.len(),
                KEY_RANGE_RECORD_LEN,
            ),
            (
                "legacy prefix keys",
                self.legacy_prefix_keys.len(),
                KEY_RANGE_RECORD_LEN,
            ),
            (
                "legacy target frequencies",
                self.legacy_target_frequencies.len(),
                STRING_U32_RECORD_LEN,
            ),
        ] {
            if len % record_len != 0 {
                return Err(LexiconError::Parse(format!(
                    "dictionary image {label} section has partial record"
                )));
            }
        }
        if self.exact_postings.len() % 4 != 0
            || self.alias_postings.len() % 4 != 0
            || self.gram_postings.len() % 4 != 0
            || self.legacy_roman_postings.len() % 4 != 0
            || self.legacy_normalized_postings.len() % 4 != 0
            || self.legacy_target_postings.len() % 4 != 0
            || self.legacy_prefix_postings.len() % 4 != 0
        {
            return Err(LexiconError::Parse(
                "dictionary image postings section has partial record".to_owned(),
            ));
        }
        Ok(())
    }
}

impl<'a> DictionaryEntryView<'a> {
    pub(crate) fn target(&self) -> Result<&'a str> {
        self.image.string(read_u32_at(self.image.entries, self.offset)?)
    }

    pub(crate) fn canonical_roman(&self) -> Result<&'a str> {
        self.image.string(read_u32_at(self.image.entries, self.offset + 4)?)
    }

    pub(crate) fn normalized_key(&self) -> Result<&'a str> {
        self.image.string(read_u32_at(self.image.entries, self.offset + 8)?)
    }

    pub(crate) fn frequency(&self) -> Result<u32> {
        read_u32_at(self.image.entries, self.offset + 12)
    }

    pub(crate) fn frequency_lang(&self) -> Result<&'a str> {
        self.image.string(read_u32_at(self.image.entries, self.offset + 16)?)
    }

    pub(crate) fn first_tag(&self) -> Result<Option<&'a str>> {
        self.image
            .optional_string(read_u32_at(self.image.entries, self.offset + 20)?)
    }

    pub(crate) fn last_tag(&self) -> Result<Option<&'a str>> {
        self.image
            .optional_string(read_u32_at(self.image.entries, self.offset + 24)?)
    }
}

pub(crate) struct DictionaryPostings<'a> {
    source: &'a [u8],
    index: usize,
}

impl Iterator for DictionaryPostings<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.index.checked_mul(4)?;
        if self.source.len().saturating_sub(offset) < 4 {
            return None;
        }
        self.index += 1;
        Some(u32::from_le_bytes(
            self.source[offset..offset + 4]
                .try_into()
                .expect("posting slice has fixed width"),
        ))
    }
}

fn section(source: &[u8], section_count: usize, id: u32) -> Result<&[u8]> {
    for index in 0..section_count {
        let offset = HEADER_LEN + index * SECTION_RECORD_LEN;
        if read_u32_at(source, offset)? != id {
            continue;
        }
        let start = read_u32_at(source, offset + 4)? as usize;
        let len = read_u32_at(source, offset + 8)? as usize;
        let range = checked_range(source, start, len)?;
        return Ok(&source[range]);
    }
    Err(LexiconError::Parse(format!("dictionary image missing section {id}")))
}

fn checked_range(source: &[u8], start: usize, len: usize) -> Result<Range<usize>> {
    let end = start
        .checked_add(len)
        .ok_or_else(|| LexiconError::Parse("dictionary image range overflow".to_owned()))?;
    if end > source.len() {
        return Err(LexiconError::Parse(
            "dictionary image section is out of bounds".to_owned(),
        ));
    }
    Ok(start..end)
}

fn read_u32_at(source: &[u8], offset: usize) -> Result<u32> {
    if source.len().saturating_sub(offset) < 4 {
        return Err(LexiconError::Parse("dictionary image payload is truncated".to_owned()));
    }
    Ok(u32::from_le_bytes(
        source[offset..offset + 4].try_into().expect("slice length checked"),
    ))
}
