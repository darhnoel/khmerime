//! Android IME adapter – JNI bridge.
//!
//! Every `#[no_mangle]` function corresponds to a `private external fun` on
//! `com.khmerime.input.KhmerImeSession`. The `ImeSession` is heap-allocated
//! via `Box::into_raw`; Kotlin stores the address as a `Long` (nativeHandle).
//! All session state lives in Rust; Kotlin never inspects it directly.

use std::collections::HashMap;

use jni::objects::{JObject, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;
use khmerime_core::{DecoderConfig, SpanProposalMode, Transliterator};
use khmerime_session::{
    ImeSession, ImeSessionOptions, InputMode, NativeKeyEvent, PhraseCandidate, SegmentPreviewEntry,
    SegmentedPreviewMode, SessionCommand, SessionResult, SessionSnapshot,
};
use serde::Serialize;

// ── Key constants ─────────────────────────────────────────────────────────────

const KEY_BACKSPACE: u32 = 0xFF08;
const KEY_RETURN: u32 = 0xFF0D;
const KEY_SPACE: u32 = 0x20;
const KEY_LEFT: u32 = 0xFF51;
const KEY_RIGHT: u32 = 0xFF53;
const KEY_TAB: u32 = 0xFF09;

fn key_event(keyval: u32) -> NativeKeyEvent {
    NativeKeyEvent {
        keyval,
        keycode: 0,
        state: 0,
    }
}

// ── Render state ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct RenderState {
    candidates: Vec<String>,
    selected_index: Option<u64>,
    preedit: String,
    segments: Vec<SegmentEntry>,
    focused_segment_index: Option<u64>,
    commit_text: Option<String>,
    segment_edit_active: bool,
    segment_edit_index: Option<u64>,
    // Ranked whole-phrase hypotheses for the Phrase Wheel (ADR-0015). The UI shows the
    // ones other than `selected_phrase_index` (which the strip previews).
    phrase_candidates: Vec<PhraseCandidateJson>,
    selected_phrase_index: u64,
}

#[derive(Serialize)]
struct PhraseCandidateJson {
    text: String,
    segments: Vec<SegmentEntry>,
}

impl From<&PhraseCandidate> for PhraseCandidateJson {
    fn from(candidate: &PhraseCandidate) -> Self {
        PhraseCandidateJson {
            text: candidate.text.clone(),
            segments: candidate
                .segments
                .iter()
                .map(|segment| SegmentEntry {
                    output: segment.output.clone(),
                    input: segment.input.clone(),
                    focused: false,
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct SegmentEntry {
    output: String,
    input: String,
    focused: bool,
}

impl From<&SegmentPreviewEntry> for SegmentEntry {
    fn from(segment: &SegmentPreviewEntry) -> Self {
        SegmentEntry {
            output: segment.output.clone(),
            input: segment.input.clone(),
            focused: segment.focused,
        }
    }
}

fn make_render_state(snapshot: &SessionSnapshot, result: &SessionResult) -> RenderState {
    RenderState {
        candidates: snapshot.candidates.clone(),
        selected_index: snapshot.selected_index.map(|i| i as u64),
        preedit: snapshot.preedit.clone(),
        segments: snapshot.segment_preview.iter().map(SegmentEntry::from).collect(),
        focused_segment_index: snapshot.focused_segment_index.map(|i| i as u64),
        commit_text: result.commit_text.clone(),
        segment_edit_active: snapshot.segment_edit_active,
        segment_edit_index: snapshot.segment_edit_index.map(|i| i as u64),
        phrase_candidates: snapshot
            .phrase_candidates
            .iter()
            .map(PhraseCandidateJson::from)
            .collect(),
        selected_phrase_index: snapshot.selected_phrase_index as u64,
    }
}

fn render_json(env: &mut JNIEnv, snapshot: &SessionSnapshot, result: &SessionResult) -> jstring {
    let json = serde_json::to_string(&make_render_state(snapshot, result)).expect("render state must serialize");
    let js = env.new_string(&json).expect("new_string must not fail");
    js.into_raw()
}

// ── Session helpers ───────────────────────────────────────────────────────────

// Smart refinement runs after a typing pause, off the keystroke hot path, so it gets a longer
// deadline than the live decoder for one provider inference.
const SMART_REFINE_MAX_LATENCY_MS: u64 = 2_000;

fn smart_refiner_config() -> DecoderConfig {
    let mut config = DecoderConfig::shadow_interactive().with_span_proposal_mode(SpanProposalMode::Model);
    config.wfst_max_latency_ms = SMART_REFINE_MAX_LATENCY_MS;
    config
}

/// Build a fresh Android session. The primary (live) engine is ALWAYS Standard — the keystroke
/// hot path never runs the model. When `smart`, a Model-mode **visible refiner** is attached; it
/// runs only via `refine_with_model` on a debounced pause, off the hot path. The model is inert
/// unless a provider was registered via `khmerime_core::register_span_proposal_provider` (paid
/// build only). Unlike iOS, Android keeps the full SearchIndex (no `no-search-index` feature).
fn build_session(smart: bool) -> ImeSession {
    let live = Transliterator::from_default_data_with_config(
        DecoderConfig::shadow_interactive().with_span_proposal_mode(SpanProposalMode::Disabled),
    )
    .expect("compiled-in lexicon must be valid");
    let mut builder = ImeSession::builder(live, HashMap::new())
        .input_mode(InputMode::Roman)
        .options(ImeSessionOptions {
            segmented_preview: SegmentedPreviewMode::Enabled,
            ..Default::default()
        });
    if smart {
        let refiner = Transliterator::from_default_data_with_config(smart_refiner_config())
            .expect("compiled-in lexicon must be valid");
        builder = builder.visible_refiner(refiner);
    }
    builder.build()
}

/// Toggle Standard/Smart by rebuilding the session in place. Smart attaches the Model-mode visible
/// refiner (the keystroke hot path stays Standard either way). The current composition is reset —
/// this is a settings action, not a mid-typing one. Inert without a registered provider; never panics.
fn set_model_mode(session: &mut ImeSession, smart: bool) {
    *session = build_session(smart);
}

/// Debounced model refine: re-decode the composition with the Model-mode visible refiner, OFF the
/// keystroke hot path. `expected_raw` is the roman the caller captured when it *scheduled* the
/// refine; the session's staleness guard (`composition_raw != expected_raw`) drops the refine if a
/// keystroke changed the composition in between, so a stale async result never renders over newer
/// input. No-op when empty; inert when Standard or no provider is registered.
fn refine_with_model(session: &mut ImeSession, expected_raw: &str) {
    if !expected_raw.is_empty() {
        session.refine_segmented_with_visible_refiner(expected_raw);
    }
}

// Safety: `handle` must be a pointer produced by `nativeCreate` that has not
// yet been freed by `nativeDestroy`.
unsafe fn session_mut(handle: jlong) -> &'static mut ImeSession {
    &mut *(handle as *mut ImeSession)
}

// ── JNI exports ───────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeCreate(_env: JNIEnv, _obj: JObject) -> jlong {
    Box::into_raw(Box::new(build_session(false))) as jlong
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeDestroy(_env: JNIEnv, _obj: JObject, handle: jlong) {
    if handle != 0 {
        unsafe { drop(Box::from_raw(handle as *mut ImeSession)) }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeFocusIn(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    s.focus_in();
    render_json(&mut env, &s.snapshot(), &SessionResult::default())
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeFocusOut(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    s.focus_out();
    render_json(&mut env, &s.snapshot(), &SessionResult::default())
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeProcessCharacter(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
    ch: JString,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let ch_str = env.get_string(&ch).expect("get_string must not fail");
    let keyval = ch_str.to_str().unwrap_or("?").chars().next().unwrap_or('?') as u32;
    let result = s.process_native_key_event(key_event(keyval));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeProcessBackspace(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_BACKSPACE));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeProcessSpace(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_SPACE));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeProcessEnter(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_RETURN));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeProcessLeft(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_LEFT));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeProcessRight(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_RIGHT));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeProcessTab(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_TAB));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeProcessDigit(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
    n: jint,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let digit = n.clamp(0, 9) as u32;
    let result = s.process_native_key_event(key_event(b'0' as u32 + digit));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeSelectPhrase(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
    index: jint,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_command(SessionCommand::SelectPhrase(index.max(0) as usize));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeEnterCharPick(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    s.process_command(SessionCommand::SetInputMode(InputMode::CharPick));
    render_json(&mut env, &s.snapshot(), &SessionResult::default())
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeSetModelMode(
    _env: JNIEnv,
    _obj: JObject,
    handle: jlong,
    smart: jboolean,
) {
    let s = unsafe { session_mut(handle) };
    set_model_mode(s, smart != 0);
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeIsModelMode(
    _env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jboolean {
    let s = unsafe { session_mut(handle) };
    s.visible_refiner_active() as jboolean
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeRefineWithModel(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
    expected_raw: JString,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let raw = env.get_string(&expected_raw).expect("get_string must not fail");
    refine_with_model(s, &raw.to_string_lossy());
    render_json(&mut env, &s.snapshot(), &SessionResult::default())
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_input_KhmerImeSession_nativeExitCharPick(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    s.process_command(SessionCommand::SetInputMode(InputMode::Roman));
    render_json(&mut env, &s.snapshot(), &SessionResult::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_session_attaches_a_model_visible_refiner() {
        let standard = build_session(false);
        assert!(
            !standard.visible_refiner_active(),
            "Standard must have no model refiner"
        );

        let smart = build_session(true);
        assert!(
            smart.visible_refiner_active(),
            "Smart must attach the Model-mode visible refiner"
        );
    }

    #[test]
    fn set_model_mode_swaps_the_session_in_place() {
        let mut s = build_session(false);
        set_model_mode(&mut s, true);
        assert!(s.visible_refiner_active(), "set_model_mode(true) must enable Smart");
        set_model_mode(&mut s, false);
        assert!(
            !s.visible_refiner_active(),
            "set_model_mode(false) must return to Standard"
        );
    }

    fn type_str(s: &mut ImeSession, text: &str) {
        for ch in text.chars() {
            s.process_native_key_event(key_event(ch as u32));
        }
    }

    #[test]
    fn refine_with_model_drops_stale_raw() {
        let mut s = build_session(true);
        s.focus_in();
        type_str(&mut s, "nhom");
        assert_eq!(s.snapshot().preedit, "nhom");
        // Scheduled against "nho" but composition is now "nhom" -> guard drops it, preedit intact.
        refine_with_model(&mut s, "nho");
        assert_eq!(
            s.snapshot().preedit,
            "nhom",
            "stale refine must not disturb the current composition"
        );
    }
}
