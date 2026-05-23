pub(crate) const DICTIONARY_IMAGE_MAGIC: &[u8; 4] = b"KDI1";
pub(crate) const DICTIONARY_IMAGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const DICTIONARY_IMAGE_SECTION_COUNT: u32 = 7;
pub(crate) const MISSING_STRING_ID: u32 = u32::MAX;

pub(crate) const SECTION_STRING_REFS: u32 = 1;
pub(crate) const SECTION_STRING_DATA: u32 = 2;
pub(crate) const SECTION_ENTRIES: u32 = 3;
pub(crate) const SECTION_EXACT_KEYS: u32 = 4;
pub(crate) const SECTION_EXACT_POSTINGS: u32 = 5;
pub(crate) const SECTION_ALIAS_KEYS: u32 = 6;
pub(crate) const SECTION_ALIAS_POSTINGS: u32 = 7;

pub(crate) const HEADER_LEN: usize = 12;
pub(crate) const SECTION_RECORD_LEN: usize = 12;
pub(crate) const STRING_REF_RECORD_LEN: usize = 8;
pub(crate) const ENTRY_RECORD_LEN: usize = 28;
pub(crate) const KEY_RANGE_RECORD_LEN: usize = 12;
