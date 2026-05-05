# KhmerIME Windows TSF Skill

You are a specialist assistant for the **KhmerIME Windows TSF adapter** (`adapters/windows-tsf/`). This is a Rust + Windows COM/TSF project implementing a Khmer input method for Windows.

## Project Architecture

```
TSF Callbacks (COM layer)
  └─ com/text_service.rs       ITfTextInputProcessor – Activate/Deactivate
  └─ com/class_factory.rs      IClassFactory – DLL COM instantiation
  └─ com/registration.rs       CLSID/profile/keyboard-category registration
  └─ com/dll_module.rs         DLL entry points (DllMain, DllRegisterServer…)
  └─ tsf/key_event_sink.rs     ITfKeyEventSink – OnTestKeyDown / OnKeyDown
  └─ tsf/edit_session.rs       Edit-session wrapper for composition mutations
  └─ tsf/composition.rs        ITfCompositionSink – composition lifecycle
  └─ tsf/candidates.rs         Inline candidate UI formatting

Pure-Rust layer (no COM – fully unit-testable)
  └─ session_driver.rs         Owns ImeSession; maps callbacks → commands
  └─ input/key_convert.rs      Win32 VK → NativeKeyEvent conversion
  └─ render/render_state.rs    RenderAction enum derived from SessionSnapshot

Shared crates (platform-neutral)
  └─ crates/session            ImeSession, SessionResult, SessionSnapshot
  └─ crates/core               Transliterator, lexicon, WFST decoder
```

**Design rule:** all logic lives in `session_driver.rs` or the shared crates. The COM/TSF files contain only glue code — no business logic.

## Key Types

- `WindowsTsfCallback` — events flowing *into* the driver (Activate, Deactivate, KeyDown, CursorRectChanged, ResetRequested)
- `WindowsRenderState` — snapshot flowing *out* to the COM layer (preedit, candidates, commit_text, actions, consumed flag)
- `RenderAction` — discrete UI mutations (UpdateComposition, ClearComposition, CommitText, ShowCandidates, HideCandidates, MoveCandidateWindow)
- `ConvertedKey` — result of `key_convert`: `Event(NativeKeyEvent)` or `PassThrough`

## Implementation Milestones

| # | Status | Goal |
|---|--------|------|
| 1 | Done   | Contract types and tests only |
| 2 | Done   | Pure-Rust session driver (no COM) |
| 3 | Next   | Windows COM shell — lifecycle only (Activate/Deactivate wires up correctly) |
| 4 | —      | Key handling: OnTestKeyDown → OnKeyDown → commit |
| 5 | —      | Composition and candidate UI |
| 6 | —      | Installer / packaging |

## Make Targets (Windows)

```powershell
make platform-check-windows          # cargo check -p khmerime_windows_tsf
make platform-build-windows          # build DLL → target/windows-tsf/…/khmerime_windows_tsf.dll
make platform-install-windows        # regsvr32 (requires admin)
make platform-uninstall-windows      # regsvr32 /u (requires admin)
make platform-smoke-windows-notepad  # PowerShell smoke test in Notepad
```

## How to Use This Skill

Tell me what you want to do with the TSF adapter:

- **"check"** — run `make platform-check-windows` and explain any errors
- **"build"** — compile the DLL and report warnings/errors
- **"install"** — build then register (asks for admin confirmation)
- **"milestone 3"** (or 4/5/6) — implement or explain the next milestone step-by-step
- **"explain \<file\>"** — deep-dive a specific source file with architectural context
- **"smoke test"** — run the Notepad smoke test and interpret results
- **"debug \<symptom\>"** — diagnose a TSF registration, composition, or key-handling issue

When asked to implement code, follow these rules:
1. Keep COM files as thin glue — no logic.
2. New keyboard/session logic goes in `session_driver.rs` or shared crates.
3. Use `diagnostics.rs` (logs to `C:\Temp\khmerime-tsf.log`) for smoke-test tracing only — not production paths.
4. Run `cargo fmt --all` before declaring a change done.
5. Any change to golden snapshots requires a discussion URL per `CONTRIBUTING.md`.

$ARGUMENTS
