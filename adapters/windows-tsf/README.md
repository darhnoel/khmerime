# khmerime_windows_tsf

Windows Text Services Framework (TSF) scaffold crate.

This package is intentionally contract-first and non-runnable in phase 1.

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

1. Add Windows TSF COM host shell for callback routing.
2. Define Rust boundary for callback/render structs.
3. Implement callback mapping to session commands.
4. Implement composition/preedit/candidate UI wiring.
5. Add Notepad/browser text-field smoke checklist.

## Debugging Checklist

- Confirm Activate/Deactivate pairing per focused context.
- Verify no duplicate key processing.
- Verify commit text emission is one-shot.
- Verify cursor updates synchronize candidate anchor.

## What Not To Edit Here

- Do not embed Dioxus runtime in adapter code.
- Do not fork transliteration/session logic into TSF crate.
- Do not alter session contracts in scaffold-only work.
