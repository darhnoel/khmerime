//! Future pure Rust driver around `khmerime_session::ImeSession`.
//!
//! This should be the first real implementation milestone after the skeleton.
//! It will let developers test Windows-style key events without COM, registry
//! writes, or TSF edit sessions.
//!
//! The driver must call `khmerime_session::ImeSession`; TSF code should not call
//! `khmerime_core` directly.

/// The first post-skeleton milestone for Windows adapter implementation.
pub const FIRST_IMPLEMENTATION_MILESTONE: &str = "pure Rust Windows session driver around ImeSession";
