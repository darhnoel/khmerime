mod config;
mod legacy;
mod manager;
mod observer;
mod types;
#[cfg(feature = "wfst-decoder")]
mod wfst;

pub use self::config::DecoderConfig;
pub(crate) use self::legacy::LegacyDecoder;
pub(crate) use self::manager::DecoderManager;
pub(crate) use self::observer::{build_shadow_observation, ShadowReport};
pub use self::observer::{ShadowMismatch, ShadowObservation, ShadowSummary};
pub use self::types::{DecodeCandidate, DecodeFailure, DecodeRequest, DecodeResult, DecodeSegment, DecoderMode};
#[cfg(feature = "wfst-decoder")]
pub(crate) use self::wfst::WfstDecoder;

pub(crate) trait Decoder: Send + Sync {
    fn name(&self) -> &'static str;
    fn decode(&self, request: &DecodeRequest<'_>) -> DecodeResult;
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) type DecodeTimer = std::time::Instant;

#[cfg(target_arch = "wasm32")]
pub(crate) type DecodeTimer = f64;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn start_decode_timer() -> DecodeTimer {
    std::time::Instant::now()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn start_decode_timer() -> DecodeTimer {
    js_sys::Date::now()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn elapsed_decode_us(started_at: DecodeTimer) -> u64 {
    started_at.elapsed().as_micros() as u64
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn elapsed_decode_us(started_at: DecodeTimer) -> u64 {
    ((js_sys::Date::now() - started_at).max(0.0) * 1_000.0).round() as u64
}
