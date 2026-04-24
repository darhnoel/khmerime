# khmerime_ios_keyboard

iOS custom-keyboard extension scaffold crate.

This package is intentionally contract-first and non-runnable in phase 1.

## Prerequisites

- Rust stable toolchain
- iOS keyboard extension knowledge (`UIInputViewController`, `UITextDocumentProxy`)
- Xcode familiarity for future native extension host wiring

## Native Callback Mapping (Planned)

| iOS callback | Session intent | Notes |
| --- | --- | --- |
| `viewDidAppear` | `focus_in` | Start keyboard composition session |
| `viewWillDisappear` | `focus_out` | Tear down composition session |
| key input handler | `process_key_event` | Main transliteration key flow |
| `selectionDidChange` | `set_cursor_location` | Cursor anchor updates |

## First 5 Contributor Tasks

1. Add Swift keyboard extension target shell and bridge hooks.
2. Define Swift/Rust bridge for callback + render structs.
3. Implement callback-to-command conversion.
4. Render candidates/preedit in keyboard-owned UI strip.
5. Add lifecycle and commit smoke checks in a sample host app.

## Debugging Checklist

- Confirm extension lifecycle callbacks are received in expected order.
- Verify no duplicate key dispatch.
- Verify `commit_text` insertion uses `textDocumentProxy.insertText` once.
- Verify cursor/selection changes refresh candidate anchor.

## What Not To Edit Here

- Do not add Dioxus runtime UI into this adapter.
- Do not duplicate transliteration logic from `crates/core`.
- Do not mutate `crates/session` contracts in scaffold-only tasks.
