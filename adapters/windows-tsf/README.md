# khmerime_windows_tsf

Windows Text Services Framework (TSF) scaffold crate.

This package is intentionally contract-first and non-runnable in phase 1.
It contains documented skeleton modules for the future TSF COM DLL shape, but it
does not implement COM exports, TSF registration, edit sessions, candidate UI, or
packaging yet.

## Prerequisites

- Rust stable toolchain
- Windows TSF familiarity (`ITfTextInputProcessor`, key event sinks)
- Ability to build/register text service COM components for future wiring

## Native Callback Mapping (Planned)

| TSF callback | Session intent | Notes |
| --- | --- | --- |
| `ITfTextInputProcessor::Activate` | `focus_in + enable` | Start text service session |
| `ITfTextInputProcessor::Deactivate` | `focus_out + disable` | End session and clear state |
| `ITfKeyEventSink::OnKeyDown` | `process_key_event` | Main transliteration path |
| context/cursor updates | `set_cursor_location` | Candidate anchor updates |

## First 5 Contributor Tasks

1. Keep the skeleton compiling cross-platform.
2. Add a pure Rust `session_driver` test around `khmerime_session::ImeSession`.
3. Add Windows key-conversion tests before COM code.
4. Add COM lifecycle logging only after the pure Rust path works.
5. Add TSF edit-session mutation only after activation and key sink callbacks work on Windows.

## Debugging Checklist

- Confirm Activate/Deactivate pairing per focused context.
- Verify no duplicate key processing.
- Verify commit text emission is one-shot.
- Verify cursor updates synchronize candidate anchor.

## What Not To Edit Here

- Do not embed Dioxus runtime in adapter code.
- Do not fork transliteration/session logic into TSF crate.
- Do not alter session contracts in scaffold-only work.
- Do not add `windows` crate bindings, COM exports, registry writes, or packaging
  targets until the relevant milestone needs them.
