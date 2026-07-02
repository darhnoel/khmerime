//! Shared `#[cfg(test)]` builders for the session module tests.
//!
//! Each behavior module keeps its own `#[cfg(test)] mod tests`, but they all
//! build sessions from the same fixtures so the test setup stays in one place.

#![cfg(test)]

use std::collections::HashMap;

use khmerime_core::{DecoderConfig, DecoderMode, Transliterator};

use crate::adapter_contract::{ImeSessionOptions, InputMode, SegmentedPreviewMode};
use crate::ime_session::ImeSession;

pub(crate) fn session() -> ImeSession {
    let fixture = "jea\tជា\nchea\tជា\ntov\tទៅ\nkhnhom\tខ្ញុំ\nkhnhom\tខ្ញំ\nfoo\tអា\nfoo\tអូ\n";
    let transliterator = Transliterator::from_tsv_str_with_config(fixture, DecoderConfig::shadow_interactive())
        .expect("fixture must parse");
    let mut session = ImeSession::new(transliterator, HashMap::new());
    session.focus_in();
    session
}

pub(crate) fn type_ascii(session: &mut ImeSession, text: &str) {
    for ch in text.chars() {
        session.process_key_event(ch as u32, 0, 0);
    }
}

/// A session whose single roman key `sa` has enough exact Khmer targets to fill
/// more than one page, so pagination and page-relative digit selection can be
/// exercised at a chosen `page_size`. Base consonants are deliberately neutral.
pub(crate) fn paged_session(page_size: usize) -> ImeSession {
    let fixture = "sa\tសា\nsa\tមា\nsa\tបា\nsa\tកា\nsa\tគា\nsa\tតា\nsa\tទា\nsa\tនា\nsa\tផា\nsa\tពា\nsa\tរា\nsa\tលា\n";
    let transliterator = Transliterator::from_tsv_str_with_config(fixture, DecoderConfig::shadow_interactive())
        .expect("fixture must parse");
    let mut session = ImeSession::builder(transliterator, HashMap::new())
        .options(ImeSessionOptions {
            page_size,
            ..Default::default()
        })
        .build();
    session.focus_in();
    session
}

pub(crate) fn flat_default_session_with_commit_refiner() -> ImeSession {
    let transliterator =
        Transliterator::from_default_data_with_config(DecoderConfig::legacy()).expect("default data must load");
    let mut commit_config = DecoderConfig::default()
        .with_mode(DecoderMode::Hybrid)
        .with_shadow_log(false);
    // The commit-refiner latency budget is a wall-clock deadline. Under the
    // parallel test runner, CPU contention can stall the decode thread long
    // enough that the stopwatch trips even though the decode itself is cheap,
    // spuriously failing these correctness assertions. These tests check the
    // refined Khmer result, not latency, so disable the budget here.
    commit_config.wfst_max_latency_ms = u64::MAX;
    let commit_refiner = Transliterator::from_default_data_with_config(commit_config).expect("default data must load");
    let mut session = ImeSession::builder(transliterator, HashMap::new())
        .commit_refiner(commit_refiner)
        .build();
    session.focus_in();
    session
}

pub(crate) fn flat_default_session_with_split_refiners() -> ImeSession {
    let transliterator =
        Transliterator::from_default_data_with_config(DecoderConfig::legacy()).expect("default data must load");
    let mut visible_config = DecoderConfig::shadow_interactive().with_mode(DecoderMode::Hybrid);
    visible_config.wfst_max_latency_ms = 75;
    let visible_refiner =
        Transliterator::from_default_data_with_config(visible_config).expect("default data must load");
    let commit_refiner = Transliterator::from_default_data_with_config(
        DecoderConfig::default()
            .with_mode(DecoderMode::Hybrid)
            .with_shadow_log(false),
    )
    .expect("default data must load");
    let mut session = ImeSession::builder(transliterator, HashMap::new())
        .visible_refiner(visible_refiner)
        .commit_refiner(commit_refiner)
        .build();
    session.focus_in();
    session
}

pub(crate) fn phase_a_session_without_segmented_preview() -> ImeSession {
    let transliterator =
        Transliterator::from_default_phase_a_data(DecoderConfig::legacy()).expect("phase-A data must load");
    let mut session = ImeSession::builder(transliterator, HashMap::new())
        .input_mode(InputMode::Roman)
        .options(ImeSessionOptions {
            segmented_preview: SegmentedPreviewMode::Disabled,
            ..Default::default()
        })
        .build();
    session.focus_in();
    session
}

pub(crate) fn segmented_default_session_like_ibus_bridge() -> ImeSession {
    let live =
        Transliterator::from_default_data_with_config(DecoderConfig::shadow_interactive()).expect("default data");
    let mut visible_config = DecoderConfig::shadow_interactive().with_mode(DecoderMode::Hybrid);
    visible_config.wfst_max_latency_ms = 75;
    let visible_refiner = Transliterator::from_default_data_with_config(visible_config).expect("default data");
    let mut commit_config = DecoderConfig::default()
        .with_mode(DecoderMode::Hybrid)
        .with_shadow_log(false);
    commit_config.wfst_max_latency_ms = 150;
    let commit_refiner = Transliterator::from_default_data_with_config(commit_config).expect("default data");
    let mut session = ImeSession::builder(live, HashMap::new())
        .visible_refiner(visible_refiner)
        .commit_refiner(commit_refiner)
        .build();
    session.focus_in();
    session
}
