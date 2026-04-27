//! Transliterator registry for the Dioxus runtime.
//!
//! The app keeps one lazily initialized engine per decoder mode. On wasm with
//! `fetch-data`, startup uses a small phase-A lexicon before swapping to full
//! compiled blobs; non-wasm builds use embedded data immediately.

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use roman_lookup::{DecoderConfig, DecoderMode, Transliterator};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EngineReadiness {
    Booting,
    LegacyReady,
    FullReady,
    Failed,
}

impl EngineReadiness {
    pub(crate) fn is_ready(self) -> bool {
        matches!(
            self,
            EngineReadiness::LegacyReady | EngineReadiness::FullReady | EngineReadiness::Failed
        )
    }
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static PHASE_A_LEGACY_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static FULL_LEGACY_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static FULL_SHADOW_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static FULL_WFST_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static FULL_HYBRID_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static FULL_ENGINE_READY: AtomicBool = AtomicBool::new(false);

#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
static LEGACY_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
static SHADOW_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
static WFST_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();
#[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
static HYBRID_TRANSLITERATOR: OnceLock<Transliterator> = OnceLock::new();

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static LEXICON_DATA: OnceLock<Vec<u8>> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static KHPOS_DATA: OnceLock<Vec<u8>> = OnceLock::new();
#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
static NEXT_WORD_DATA: OnceLock<Vec<u8>> = OnceLock::new();

pub(crate) fn engine(mode: DecoderMode) -> &'static Transliterator {
    #[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
    {
        if FULL_ENGINE_READY.load(Ordering::Acquire) {
            return full_engine(mode);
        }
        return PHASE_A_LEGACY_TRANSLITERATOR.get_or_init(init_phase_a_legacy_transliterator);
    }
    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    {
        match mode {
            DecoderMode::Legacy => LEGACY_TRANSLITERATOR.get_or_init(|| {
                Transliterator::from_default_data_with_config(DecoderConfig::default())
                    .expect("embedded lexicon must load")
            }),
            DecoderMode::Shadow => SHADOW_TRANSLITERATOR.get_or_init(|| {
                Transliterator::from_default_data_with_config(
                    DecoderConfig::shadow_interactive().with_shadow_log(false),
                )
                .expect("embedded lexicon must load")
            }),
            DecoderMode::Wfst => WFST_TRANSLITERATOR.get_or_init(|| {
                Transliterator::from_default_data_with_config(
                    DecoderConfig::default()
                        .with_mode(DecoderMode::Wfst)
                        .with_shadow_log(false),
                )
                .expect("embedded lexicon must load")
            }),
            DecoderMode::Hybrid => HYBRID_TRANSLITERATOR.get_or_init(|| {
                Transliterator::from_default_data_with_config(
                    DecoderConfig::default()
                        .with_mode(DecoderMode::Hybrid)
                        .with_shadow_log(false),
                )
                .expect("embedded lexicon must load")
            }),
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn init_phase_a_legacy_transliterator() -> Transliterator {
    let lexicon = LEXICON_DATA
        .get()
        .expect("phase-A lexicon data must be fetched before engine init");
    Transliterator::from_phase_a_bytes(lexicon, DecoderConfig::default()).expect("phase-A transliterator init failed")
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn init_full_transliterator(config: DecoderConfig) -> Transliterator {
    let lexicon = LEXICON_DATA
        .get()
        .expect("lexicon data must be fetched before full engine init");
    let khpos = KHPOS_DATA
        .get()
        .expect("khpos data must be fetched before full engine init");
    let next_word = NEXT_WORD_DATA
        .get()
        .expect("next-word data must be fetched before full engine init");
    Transliterator::from_compiled_bytes(lexicon, khpos, next_word, config).expect("full transliterator init failed")
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
fn full_engine(mode: DecoderMode) -> &'static Transliterator {
    match mode {
        DecoderMode::Legacy => {
            FULL_LEGACY_TRANSLITERATOR.get_or_init(|| init_full_transliterator(DecoderConfig::default()))
        }
        DecoderMode::Shadow => FULL_SHADOW_TRANSLITERATOR
            .get_or_init(|| init_full_transliterator(DecoderConfig::shadow_interactive().with_shadow_log(false))),
        DecoderMode::Wfst => FULL_WFST_TRANSLITERATOR.get_or_init(|| {
            init_full_transliterator(
                DecoderConfig::default()
                    .with_mode(DecoderMode::Wfst)
                    .with_shadow_log(false),
            )
        }),
        DecoderMode::Hybrid => FULL_HYBRID_TRANSLITERATOR.get_or_init(|| {
            init_full_transliterator(
                DecoderConfig::default()
                    .with_mode(DecoderMode::Hybrid)
                    .with_shadow_log(false),
            )
        }),
    }
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
pub(crate) fn store_phase_a_assets(lexicon: Vec<u8>, transliterator: Transliterator) {
    let _ = PHASE_A_LEGACY_TRANSLITERATOR.set(transliterator);
    let _ = LEXICON_DATA.set(lexicon);
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
pub(crate) fn store_full_support_data(khpos: Vec<u8>, next_word: Vec<u8>) {
    let _ = KHPOS_DATA.set(khpos);
    let _ = NEXT_WORD_DATA.set(next_word);
}

#[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
pub(crate) fn promote_full_engine() {
    let _ = full_engine(DecoderMode::Legacy);
    FULL_ENGINE_READY.store(true, Ordering::Release);
}

pub(crate) fn current_engine_readiness() -> EngineReadiness {
    #[cfg(all(target_arch = "wasm32", feature = "fetch-data"))]
    {
        if FULL_ENGINE_READY.load(Ordering::Acquire) {
            EngineReadiness::FullReady
        } else if PHASE_A_LEGACY_TRANSLITERATOR.get().is_some() {
            EngineReadiness::LegacyReady
        } else {
            EngineReadiness::Booting
        }
    }
    #[cfg(not(all(target_arch = "wasm32", feature = "fetch-data")))]
    {
        if LEGACY_TRANSLITERATOR.get().is_some() {
            EngineReadiness::FullReady
        } else {
            EngineReadiness::Booting
        }
    }
}
