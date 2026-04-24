# khmerime_android_ime

Android IME scaffold crate for future `InputMethodService` integration.

This package is intentionally contract-first and non-runnable in phase 1.

## Prerequisites

- Rust stable toolchain
- Android IME development familiarity (`InputMethodService`, `InputConnection`)
- Ability to build and run Android projects (Gradle/Android Studio) for future native wiring

## Native Callback Mapping (Planned)

| Android callback | Session intent | Notes |
| --- | --- | --- |
| `onStartInput` | `focus_in + enable` | Start composition for current editor |
| `onFinishInput` | `focus_out` | Clear transient composition |
| key input handler | `process_key_event` | Main transliteration path |
| `onUpdateSelection` | `set_cursor_location` | Candidate/segment preview anchor |

## First 5 Contributor Tasks

1. Add Kotlin service shell that forwards lifecycle and key events to this crate.
2. Define JNI/FFI boundary for `AndroidImeCallback` and render outputs.
3. Implement `map_callback_to_session_command` with real event conversion.
4. Add adapter contract tests for lifecycle and commit behavior.
5. Add manual smoke checklist for Android emulator/device.

## Debugging Checklist

- Confirm `onStartInput`/`onFinishInput` lifecycle ordering.
- Verify key events are not double-dispatched.
- Verify `commit_text` is sent exactly once.
- Verify candidate/preedit rendering updates from `SessionSnapshot`.

## What Not To Edit Here

- Do not move transliteration logic into this adapter.
- Do not embed Dioxus runtime/UI in adapter code.
- Do not change `crates/session` contracts from this scaffold phase.
