# khmerime_macos_imk

macOS InputMethodKit scaffold crate.

This package is intentionally contract-first and non-runnable in phase 1.

## Prerequisites

- Rust stable toolchain
- macOS InputMethodKit knowledge (`IMKInputController` lifecycle/event model)
- Ability to create macOS IME bundles for future native wiring

## Native Callback Mapping (Planned)

| IMK callback | Session intent | Notes |
| --- | --- | --- |
| `activateServer:` | `focus_in` | Start active input-client session |
| `deactivateServer:` | `focus_out` | End and clear composition |
| `handleEvent:` | `process_key_event` | Main transliteration key path |
| cursor/selection updates | `set_cursor_location` | Candidate anchor updates |

## First 5 Contributor Tasks

1. Add Objective-C/Swift IMK host shell and service bundle structure.
2. Define Rust bridge boundary for callback/render structs.
3. Implement callback mapping to session commands.
4. Wire marked text and commit text behavior.
5. Add manual macOS editor smoke matrix.

## Debugging Checklist

- Confirm `activateServer`/`deactivateServer` are paired.
- Verify key events are forwarded exactly once.
- Verify commit text is emitted once per session commit.
- Verify candidate/preedit refresh after cursor movement.

## What Not To Edit Here

- Do not embed Dioxus runtime code in adapter.
- Do not copy core decoder logic into this crate.
- Do not change session contracts during scaffold phase.
