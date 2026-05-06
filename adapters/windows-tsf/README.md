# khmerime_windows_tsf

Windows Text Services Framework (TSF) adapter crate.

This package builds a TSF COM DLL for KhmerIME. It owns Windows activation,
registration, key event routing, edit sessions, candidate UI, and packaging
plumbing while shared IME behavior stays in `khmerime_session` and
`khmerime_core`.

## Prerequisites

- Rust stable toolchain
- `x86_64-pc-windows-msvc` Rust target for native Windows builds
- WiX Toolset on `PATH` as `wix` for MSI packaging
- Windows TSF familiarity (`ITfTextInputProcessor`, key event sinks)
- Ability to build/register text service COM components

## Packaging

Build the unsigned x64 MSI from the repository root:

```powershell
make windows-package
```

The package is written to `dist/windows/KhmerIME-<version>-x64.msi`.
It installs `khmerime_windows_tsf.dll` under `Program Files\KhmerIME` and
registers/unregisters the TSF profile through the DLL's COM registration exports.

## Native Callback Mapping

| TSF callback | Session intent | Notes |
| --- | --- | --- |
| `ITfTextInputProcessor::Activate` | `focus_in + enable` | Start text service session |
| `ITfTextInputProcessor::Deactivate` | `focus_out + disable` | End session and clear state |
| `ITfKeyEventSink::OnKeyDown` | `process_key_event` | Main transliteration path |
| context/cursor updates | `set_cursor_location` | Candidate anchor updates |

## Contributor Tasks

1. Keep Windows TSF behavior behind the adapter boundary.
2. Run focused Windows checks before changing registration or edit-session code.
3. Keep packaging changes separate from decoder, ranking, and lexicon behavior.

## Debugging Checklist

- Confirm Activate/Deactivate pairing per focused context.
- Verify no duplicate key processing.
- Verify commit text emission is one-shot.
- Verify cursor updates synchronize candidate anchor.

## What Not To Edit Here

- Do not embed Dioxus runtime in adapter code.
- Do not fork transliteration/session logic into TSF crate.
- Do not alter session contracts in scaffold-only work.
- Do not change package/registration plumbing and decoder behavior in the same patch.
