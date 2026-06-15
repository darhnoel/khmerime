//! Android IME adapter – JNI bridge.
//!
//! Every `#[no_mangle]` function corresponds to a `private external fun` on
//! `com.khmerime.KhmerImeSession`. The `ImeSession` is heap-allocated
//! via `Box::into_raw`; Kotlin stores the address as a `Long` (nativeHandle).
//! All session state lives in Rust; Kotlin never inspects it directly.

use std::collections::HashMap;

use jni::objects::{JObject, JString};
use jni::sys::{jlong, jstring};
use jni::JNIEnv;
use khmerime_core::{DecoderConfig, Transliterator};
use khmerime_session::{
    ImeSession, ImeSessionOptions, InputMode, NativeKeyEvent, SegmentedPreviewMode,
    SessionResult, SessionSnapshot,
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
    NativeKeyEvent { keyval, keycode: 0, state: 0 }
}

// ── Render state ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct RenderState {
    candidates: Vec<String>,
    selected_index: Option<u64>,
    preedit: String,
    commit_text: Option<String>,
}

fn make_render_state(snapshot: &SessionSnapshot, result: &SessionResult) -> RenderState {
    RenderState {
        candidates: snapshot.candidates.clone(),
        selected_index: snapshot.selected_index.map(|i| i as u64),
        preedit: snapshot.preedit.clone(),
        commit_text: result.commit_text.clone(),
    }
}

fn render_json(env: &mut JNIEnv, snapshot: &SessionSnapshot, result: &SessionResult) -> jstring {
    let json = serde_json::to_string(&make_render_state(snapshot, result))
        .expect("render state must serialize");
    let js = env.new_string(&json).expect("new_string must not fail");
    js.into_raw()
}

// ── Session helpers ───────────────────────────────────────────────────────────

fn build_session() -> ImeSession {
    let transliterator =
        Transliterator::from_default_data_with_config(DecoderConfig::shadow_interactive())
            .expect("compiled-in lexicon must be valid");
    ImeSession::builder(transliterator, HashMap::new())
        .input_mode(InputMode::Roman)
        .options(ImeSessionOptions {
            segmented_preview: SegmentedPreviewMode::Enabled,
        })
        .build()
}

// Safety: `handle` must be a pointer produced by `nativeCreate` that has not
// yet been freed by `nativeDestroy`.
unsafe fn session_mut(handle: jlong) -> &'static mut ImeSession {
    &mut *(handle as *mut ImeSession)
}

// ── JNI exports ───────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeCreate(
    _env: JNIEnv,
    _obj: JObject,
) -> jlong {
    Box::into_raw(Box::new(build_session())) as jlong
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeDestroy(
    _env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) {
    if handle != 0 {
        unsafe { drop(Box::from_raw(handle as *mut ImeSession)) }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeFocusIn(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    s.focus_in();
    render_json(&mut env, &s.snapshot(), &SessionResult::default())
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeFocusOut(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    s.focus_out();
    render_json(&mut env, &s.snapshot(), &SessionResult::default())
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeProcessCharacter(
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
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeProcessBackspace(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_BACKSPACE));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeProcessSpace(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_SPACE));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeProcessEnter(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_RETURN));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeProcessLeft(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_LEFT));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeProcessRight(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_RIGHT));
    render_json(&mut env, &s.snapshot(), &result)
}

#[no_mangle]
pub extern "C" fn Java_com_khmerime_KhmerImeSession_nativeProcessTab(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) -> jstring {
    let s = unsafe { session_mut(handle) };
    let result = s.process_native_key_event(key_event(KEY_TAB));
    render_json(&mut env, &s.snapshot(), &result)
}
