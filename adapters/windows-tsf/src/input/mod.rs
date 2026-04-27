//! Future Windows key conversion boundary.
//!
//! This module should convert Windows virtual-key/character data into
//! `khmerime_session::NativeKeyEvent`. It must not call `khmerime_core` or own
//! Khmer-specific ranking behavior.

pub mod key_convert;
