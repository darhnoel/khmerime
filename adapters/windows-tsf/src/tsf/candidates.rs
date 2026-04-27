//! Placeholder for TSF candidate UI mapping.
//!
//! Future code should convert `SessionSnapshot.candidates` and
//! `SessionSnapshot.selected_index` into the Windows candidate UI model. Do not
//! rank or filter candidates here.

/// Source fields for future candidate rendering.
pub const CANDIDATE_SOURCE_FIELDS: &[&str] = &["SessionSnapshot.candidates", "SessionSnapshot.selected_index"];
