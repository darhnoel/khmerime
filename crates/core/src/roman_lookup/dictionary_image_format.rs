pub(crate) const DICTIONARY_IMAGE_MAGIC: &[u8; 4] = b"KDI1";
pub(crate) const DICTIONARY_IMAGE_SCHEMA_VERSION: u32 = 4;
pub(crate) const DICTIONARY_IMAGE_SECTION_COUNT: u32 = 20;
pub(crate) const MISSING_STRING_ID: u32 = u32::MAX;

pub(crate) const SECTION_STRING_REFS: u32 = 1;
pub(crate) const SECTION_STRING_DATA: u32 = 2;
pub(crate) const SECTION_ENTRIES: u32 = 3;
pub(crate) const SECTION_EXACT_KEYS: u32 = 4;
pub(crate) const SECTION_EXACT_POSTINGS: u32 = 5;
pub(crate) const SECTION_ALIAS_KEYS: u32 = 6;
pub(crate) const SECTION_ALIAS_POSTINGS: u32 = 7;
pub(crate) const SECTION_GRAM_KEYS: u32 = 8;
pub(crate) const SECTION_GRAM_POSTINGS: u32 = 9;
pub(crate) const SECTION_LEGACY_ROMAN_KEYS: u32 = 10;
pub(crate) const SECTION_LEGACY_ROMAN_POSTINGS: u32 = 11;
pub(crate) const SECTION_LEGACY_NORMALIZED_KEYS: u32 = 12;
pub(crate) const SECTION_LEGACY_NORMALIZED_POSTINGS: u32 = 13;
pub(crate) const SECTION_LEGACY_TARGET_KEYS: u32 = 14;
pub(crate) const SECTION_LEGACY_TARGET_POSTINGS: u32 = 15;
pub(crate) const SECTION_LEGACY_PREFIX_KEYS: u32 = 16;
pub(crate) const SECTION_LEGACY_PREFIX_POSTINGS: u32 = 17;
pub(crate) const SECTION_LEGACY_TARGET_FREQUENCIES: u32 = 18;
// Per-entry alias keys (the entry's score_forms minus its normalized key), so the
// ranked entry table can be served from the image instead of a heap Vec. REFS is a
// dense (start, count) record per entry id; IDS is a flat u32 array of string ids.
pub(crate) const SECTION_ENTRY_ALIAS_REFS: u32 = 19;
pub(crate) const SECTION_ENTRY_ALIAS_IDS: u32 = 20;

pub(crate) const HEADER_LEN: usize = 12;
pub(crate) const SECTION_RECORD_LEN: usize = 12;
pub(crate) const STRING_REF_RECORD_LEN: usize = 8;
pub(crate) const ENTRY_RECORD_LEN: usize = 28;
pub(crate) const ENTRY_ALIAS_REF_RECORD_LEN: usize = 8;
pub(crate) const KEY_RANGE_RECORD_LEN: usize = 12;
pub(crate) const STRING_U32_RECORD_LEN: usize = 8;
