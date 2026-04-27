# Windows Khmer IME With TSF

This document explains how to develop the Windows KhmerIME adapter using the Windows Text Services Framework (TSF).

For the shared platform workflow, read [`docs/platforms/README.md`](README.md).

The short version: Windows TSF should only be the native shell. KhmerIME behavior should stay in the shared Rust session and core engine.

```text
Windows focused text app
  -> TSF manager
  -> KhmerIME TSF text service COM object
  -> Windows adapter callback layer
  -> khmerime_session::ImeSession
  -> khmerime_core::Transliterator
  -> SessionSnapshot + SessionResult
  -> TSF composition/candidate/commit UI
```

Do not embed the Dioxus app in the Windows adapter. Do not fork transliteration logic into Windows-specific code.

## Current Status

The Windows adapter is currently a documentation and skeleton phase only. It is
not a runnable Windows IME yet: it does not export a COM DLL, register a TSF
profile, mutate TSF document ranges, or render native candidate UI.

The current scaffold is intentionally cross-platform compilable so Linux/macOS
development checks can validate the contract without Windows APIs installed:

```text
adapters/windows-tsf/
  Cargo.toml
  README.md
  src/
    lib.rs
    history.rs
    session_driver.rs
    com/          Windows-only placeholder modules
    input/        documented key-conversion boundary
    render/       documented render-action boundary
    tsf/          Windows-only placeholder modules
```

`adapters/windows-tsf/src/lib.rs` is a contract skeleton. It documents the callback surface and render responsibilities but does not yet register a real Windows COM text service.

The current crate exposes:

```text
WindowsTsfCallback
  Logical callback events from TSF.

CallbackMapping
  Static mapping from TSF callbacks to khmerime_session intent.

WindowsRenderState
  Adapter-owned render state derived from SessionSnapshot + SessionResult.

callback_map()
  Static mapping used by docs/tests.

map_callback_to_session_command(...)
  Placeholder for future TSF -> SessionCommand conversion.

derive_render_state(...)
  Converts session output into preedit/candidates/commit responsibilities.
```

The first real implementation after this skeleton should be a pure Rust
`session_driver` test around `khmerime_session::ImeSession`, still before COM
registration or TSF edit-session mutation.

## What TSF Owns

TSF is the Windows-native input framework. For KhmerIME, the TSF adapter should own only Windows integration work:

- COM registration and activation.
- Text-service lifecycle.
- Keyboard event sinks.
- Conversion from Windows virtual-key events to `NativeKeyEvent`.
- TSF composition/preedit display.
- TSF candidate UI display.
- One-shot commit into the focused document.
- Cursor/context geometry for candidate anchoring.
- Windows install/uninstall packaging later.

TSF should not own:

- roman normalization,
- Khmer candidate ranking,
- phrase segmentation,
- history learning policy,
- decoder selection,
- Dioxus UI behavior.

Those belong to `crates/core` and `crates/session`.

## Microsoft TSF Interfaces To Start With

Use the official Microsoft docs as the API source of truth:

- TSF overview: <https://learn.microsoft.com/en-us/windows/win32/tsf/text-services-framework>
- `ITfTextInputProcessor`: <https://learn.microsoft.com/en-us/windows/win32/api/msctf/nn-msctf-itftextinputprocessor>
- `ITfKeyEventSink`: <https://learn.microsoft.com/en-us/windows/win32/api/msctf/nn-msctf-itfkeyeventsink>
- `ITfKeystrokeMgr::AdviseKeyEventSink`: <https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfkeystrokemgr-advisekeyeventsink>
- `ITfKeyEventSink::OnTestKeyDown`: <https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfkeyeventsink-ontestkeydown>
- `ITfKeyEventSink::OnKeyDown`: <https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfkeyeventsink-onkeydown>
- `ITfThreadMgrEventSink`: <https://learn.microsoft.com/en-us/windows/win32/api/msctf/nn-msctf-itfthreadmgreventsink>

The core TSF idea is that KhmerIME becomes a text service. Windows activates it through TSF, then the text service receives lifecycle and keyboard callbacks and decides whether to consume keys, update composition, show candidates, or commit text.

## Repo Architecture For Windows

The intended Windows path is TSF-native. The Windows adapter should call the shared Rust session layer directly:

```text
TSF COM adapter
  -> ImeSession
  -> Transliterator
```

The first implementation should keep the adapter Rust-native and call `khmerime_session` from the TSF adapter crate. Add a process boundary only if a concrete Windows packaging, crash-isolation, or language-binding need appears later.

The maintained boundary should be:

```text
adapters/windows-tsf
  owns Windows COM/TSF glue and render mapping only

crates/session
  owns platform-neutral composition state and key semantics

crates/core
  owns transliteration, segmentation, ranking, and decoder behavior
```

## Implementation Architecture Guide

Use Mozc's Windows `src/win32/tip` area as an architecture reference, not as a
complete template to copy. Mozc separates TSF runtime responsibilities into text
service, class factory, edit sessions, key handling, composition utilities,
candidate lists, display attributes, and UI handlers. KhmerIME should borrow
that separation while keeping the first implementation much smaller.

In the current phase, the files below are skeleton boundaries with documentation
and placeholder constants only. They are not allowed to pretend to register,
activate, edit, or package a working Windows IME.

Recommended adapter shape:

```text
adapters/windows-tsf/src/
  lib.rs
    Public adapter contract and test-facing exports.

  com/
    dll_module.rs
      DLL entry points, module lifetime, class-object lookup.

    class_factory.rs
      COM class factory for the KhmerIME text service.

    registration.rs
      Register/unregister COM class, TSF text service profile, icons, and
      language/profile metadata.

    text_service.rs
      `ITfTextInputProcessor` implementation. Owns activation, deactivation,
      thread manager references, client id, and sink registration.

  tsf/
    thread_context.rs
      Per-thread TSF state: thread manager, document manager, active context,
      sink cookies, and current focus state.

    key_event_sink.rs
      `ITfKeyEventSink` implementation. Owns `OnTestKeyDown`, `OnKeyDown`,
      `OnKeyUp`, and `OnSetFocus` routing.

    edit_session.rs
      Safe wrapper for TSF edit sessions. All document mutations, composition
      updates, and commits should pass through this layer.

    composition.rs
      Start/update/end TSF composition from `SessionSnapshot.preedit`.

    candidates.rs
      Convert `SessionSnapshot.candidates` and selected index into a TSF
      candidate UI model.

    display_attributes.rs
      TSF display attributes for preedit/converted text if needed after v1.

  input/
    key_convert.rs
      Convert Windows virtual-key and character data into `NativeKeyEvent`.

  render/
    render_state.rs
      Convert `SessionSnapshot` + `SessionResult` into Windows render actions.

  session_driver.rs
    Pure Rust glue that owns `ImeSession` and receives adapter events. This is
    testable without COM and should be implemented before real TSF mutation.

  history.rs
    Windows user-local history store, eventually `%APPDATA%\khmerime\history.tsv`.
```

The main runtime should stay linear:

```text
TSF callback
  -> input/key_convert.rs or lifecycle mapping
  -> session_driver.rs
  -> khmerime_session::ImeSession
  -> render/render_state.rs
  -> tsf/edit_session.rs
  -> tsf/composition.rs + tsf/candidates.rs + commit action
```

Do not let TSF code call `khmerime_core` directly. The adapter should go through
`khmerime_session` so behavior stays shared with Linux, web, CLI, and future
mobile adapters.

Do not add the `windows` crate dependency, `cdylib`, exported COM symbols,
registry writes, installer scripts, or `make windows-package` while this remains
a skeleton-only phase.

### Mozc Lessons To Borrow

- Keep `ITfTextInputProcessor` focused on activation, deactivation, and sink
  ownership. Do not put candidate ranking or key semantics there.
- Keep key handling in a dedicated key-event sink module. `OnTestKeyDown` and
  `OnKeyDown` must share the same prediction/conversion logic.
- Use edit-session objects for document mutation. TSF text changes should not be
  scattered across callback methods.
- Keep composition utilities separate from candidate UI utilities.
- Keep package/registration code separate from runtime typing behavior.
- Add small tests around conversion, candidate-list mapping, and render-state
  derivation before adding more COM code.

### Mozc Complexity To Avoid For V1

- Do not implement IMM32. Target TSF only.
- Do not add broker, server, renderer, or cache-service processes unless a real
  crash-isolation or packaging need appears.
- Do not build a custom renderer before the TSF composition and candidate path
  works with basic UI.
- Do not implement lang bar menus, reconversion, surrounding-text prediction, or
  advanced preserved-key handling before basic typing is stable.
- Do not add MSI/WiX packaging until the TSF service can be manually registered,
  selected, and used in Notepad.

## Lifecycle Mapping

| TSF callback | KhmerIME session intent | Notes |
| --- | --- | --- |
| `ITfTextInputProcessor::Activate` | create/load `ImeSession`, `Enable`, `FocusIn` | Called when TSF activates the text service for a thread. |
| `ITfTextInputProcessor::Deactivate` | `FocusOut`, `Disable`, save history | Clear composition and release TSF sinks. |
| `ITfKeystrokeMgr::AdviseKeyEventSink` | register key sink | Install the text service as the keyboard event sink. |
| `ITfKeyEventSink::OnSetFocus(TRUE)` | `FocusIn` | Focus entered the text service. |
| `ITfKeyEventSink::OnSetFocus(FALSE)` | `FocusOut` | Reset composition on focus loss. |
| `ITfKeyEventSink::OnTestKeyDown` | predict whether key may be consumed | Should be consistent with actual `OnKeyDown`. |
| `ITfKeyEventSink::OnKeyDown` | `ProcessKeyEvent(NativeKeyEvent)` | Main transliteration path. |
| context/cursor update | `SetCursorLocation(CursorLocation)` | Used to anchor candidate UI near caret. |
| external reset/profile change | `Reset` | Clear preedit/candidates. |

## Key Event Flow

The key-event path should be deterministic.

```text
OnTestKeyDown(wParam, lParam)
  -> convert Windows key data enough to know if KhmerIME may handle it
  -> return pfEaten = TRUE for keys KhmerIME would consume

OnKeyDown(wParam, lParam)
  -> convert into NativeKeyEvent
  -> session.process_command(ProcessKeyEvent(event))
  -> inspect SessionResult
  -> update TSF composition/candidates from SessionSnapshot
  -> if result.commit_text is Some(text), commit exactly once
  -> set pfEaten = result.consumed
```

The important rule: `OnTestKeyDown` and `OnKeyDown` must agree. If `OnTestKeyDown` says KhmerIME will eat a key, then `OnKeyDown` should normally consume it too. Do not let the host app receive a roman key that the session already consumed.

## Mapping Windows Keys To `NativeKeyEvent`

`khmerime_session::NativeKeyEvent` has this shape:

```rust
pub struct NativeKeyEvent {
    pub keyval: u32,
    pub keycode: u32,
    pub state: u32,
}
```

For the first Windows implementation:

- `keyval` should be the Unicode scalar value for printable text when available.
- `keycode` should carry the Windows virtual-key code from `wParam`.
- `state` should encode modifier state consistently enough for `ImeSession` to ignore control/alt/meta-style shortcuts.

Do not blindly pass virtual-key codes as printable characters. For example, the `A` key and typed `a` are not the same thing once keyboard layout, shift, caps lock, and IME mode are considered.

For v1, keep key conversion conservative:

- pass printable ASCII only when it is clearly text input,
- pass Enter, Backspace, Escape, Space, Left, Right, Up, Down using the session's expected key semantics,
- pass Ctrl/Alt/Windows shortcuts through to the application,
- ignore key-up events unless a future feature needs them.

The current session API expects numeric `keyval` values for special keys. The Windows adapter should translate Windows virtual keys into the session's expected special-key values in one small conversion module. If that mapping becomes hard to maintain, introduce a shared platform-neutral key enum in `crates/session` as a separate refactor.

## Session Behavior The Adapter Gets For Free

Once Windows key events are mapped into `ImeSession`, the adapter gets existing behavior from `crates/session`:

- roman preedit accumulation,
- candidate generation,
- candidate cycling,
- number-key candidate selection,
- Enter commit,
- Backspace edit,
- Escape cancel,
- segmented phrase mode,
- segment focus movement,
- segment candidate cycling,
- one-shot `commit_text`,
- history learning hooks.

The Windows adapter should render these states; it should not reimplement them.

## Rendering TSF Composition And Candidates

After every consumed key or lifecycle update, derive render state from:

```text
SessionSnapshot
SessionResult
```

Important fields:

```text
snapshot.preedit
  Text currently being composed. Render this as TSF composition/marked text.

snapshot.raw_preedit
  Raw roman input. Useful for debugging and future candidate labels.

snapshot.candidates
  Current visible candidate strings.

snapshot.candidate_display
  Candidate metadata such as recommended marker and roman hints.

snapshot.selected_index
  Active candidate index.

snapshot.segmented_active
  Whether long-token segmented phrase refinement is active.

snapshot.segment_preview
  Per-segment preview data for future Windows candidate/auxiliary UI.

result.consumed
  Whether the original key should be eaten.

result.commit_text
  Final text to commit exactly once.
```

Initial v1 rendering target:

1. Show `snapshot.preedit` as Windows composition text.
2. Show `snapshot.candidates` in TSF candidate UI.
3. Highlight `snapshot.selected_index` if present.
4. Commit `result.commit_text` once and then clear composition if the session reset.
5. If segmented mode is active, first show the composed phrase as preedit and the focused segment candidates in the candidate list.

Do not block v1 on a beautiful segment preview. A correct preedit + candidate list + commit path is more important.

## History Persistence

Windows should eventually have its own history store under a user-local app data directory such as:

```text
%APPDATA%\khmerime\history.tsv
```

Keep the history file format compatible with the `HistoryStore` trait from `crates/session` so ranking behavior remains shared across platforms.

For the first TSF spike, it is acceptable to start with in-memory history only. Add file persistence after key handling and commit flow are stable.

## Development Milestones

### Milestone 1: Contract Tests Only

Goal: keep the scaffold honest before writing COM glue.

Tasks:

1. Add unit tests in `adapters/windows-tsf` for `callback_map()`.
2. Add tests for `derive_render_state()`.
3. Add a placeholder test proving `map_callback_to_session_command()` returns `None` until implemented.
4. Run `cargo test -p khmerime_windows_tsf`.

### Milestone 2: Local Session Driver

Goal: verify Windows-style callback conversion without COM registration.

Tasks:

1. Implement a pure Rust driver function that owns `ImeSession`.
2. Feed synthetic `WindowsTsfCallback::KeyDown(...)` events.
3. Verify `jea + Enter` produces Khmer commit text.
4. Verify Backspace, Escape, Space cycling, arrow cycling, and number selection.
5. Add tests that do not require Windows.

This milestone should still avoid COM. It proves the adapter contract can drive the shared session.

### Milestone 3: Windows COM Shell

Goal: create a real TSF text service that can be loaded by Windows.

Tasks:

1. Add COM class identity and registration metadata.
2. Implement `ITfTextInputProcessor::Activate` and `Deactivate`.
3. During activation, obtain the thread manager/client id required for key sink registration.
4. Register `ITfKeyEventSink` through `ITfKeystrokeMgr::AdviseKeyEventSink`.
5. Unadvise/release sinks during deactivation.
6. Log lifecycle events before attempting text mutation.

Keep this milestone focused on lifecycle and loading. Do not combine it with full candidate UI.

### Milestone 4: Key Handling And Commit

Goal: type roman text in Notepad and commit Khmer text.

Tasks:

1. Convert `wParam`/`lParam` into `NativeKeyEvent`.
2. Implement `OnTestKeyDown` and `OnKeyDown` consistently.
3. Call `ImeSession` on key down.
4. Set `pfEaten` from `SessionResult.consumed`.
5. Commit `SessionResult.commit_text` exactly once.
6. Verify raw fallback behavior for unmatched roman input.

Manual smoke target:

```text
Open Notepad
Switch to KhmerIME TSF profile
Type: jea
Press Enter
Expected: Khmer candidate is committed once, roman text is not duplicated
```

### Milestone 5: Composition And Candidate UI

Goal: make the IME usable before commit.

Tasks:

1. Render `snapshot.preedit` as composition text.
2. Render `snapshot.candidates` in a candidate UI.
3. Highlight `snapshot.selected_index`.
4. Update UI after Space, Up/Down, Left/Right, Backspace, and Escape.
5. Keep candidate UI anchored near the current text context/caret.
6. Verify segmented phrase mode with a long token such as `khnhomttov`.

### Milestone 6: Installer And Packaging

Goal: let non-developer Windows users install and test.

Do not add `make windows-package` yet. Add Windows packaging only after a real
TSF COM text service exists and can be registered, selected, and used on Windows.

Tasks:

1. Decide package format, likely MSI via WiX or an equivalent signed installer.
2. Register/unregister the TSF COM text service.
3. Include architecture-specific binaries.
4. Add uninstall cleanup.
5. Document how to enable the input method in Windows language/input settings.
6. Add a manual smoke checklist for Notepad, Chromium textarea, and a Microsoft Office text field.

## First Contributor Tasks

Start with these in order:

1. Add tests for the existing scaffold in `adapters/windows-tsf`.
2. Implement a pure Rust session-driver layer before COM.
3. Add a Windows key conversion module with explicit tests.
4. Implement COM lifecycle logging only.
5. Add key sink registration.
6. Add key down -> session -> commit flow.
7. Add composition/preedit rendering.
8. Add candidate rendering.
9. Add history persistence.
10. Add packaging.

## Manual Smoke Checklist

Use this after a real TSF text service exists:

```text
Install/register KhmerIME TSF service
Open Notepad
Switch input method to KhmerIME
Type jea
Verify preedit/candidates appear
Press Space
Verify selected candidate changes
Press Enter
Verify Khmer text commits once
Type khnhomttov
Verify segmented phrase behavior is usable
Press Escape during composition
Verify preedit clears and host app does not receive garbage text
Switch away from KhmerIME
Verify normal English typing passes through
Uninstall/unregister service
Verify input method disappears from Windows settings
```

## Debugging Checklist

When something breaks, isolate the layer:

```text
Activation not called
  -> COM registration/profile issue.

Activation works, key callbacks missing
  -> key sink registration or foreground sink issue.

Key callbacks fire, host also receives roman text
  -> pfEaten/result.consumed mismatch.

Commit appears twice
  -> commit_text handled more than once or composition not cleared.

Preedit sticks after focus loss
  -> Deactivate/OnSetFocus(FALSE) is not calling FocusOut/Reset.

Candidates wrong but key flow works
  -> core/session behavior, not TSF UI. Reproduce with CLI or session tests.

Candidate window in wrong place
  -> context/cursor rectangle mapping issue.
```

## What Not To Do

- Do not embed the Dioxus app as the Windows IME.
- Do not duplicate transliteration or ranking in the Windows crate.
- Do not make COM registration and decoder behavior changes in the same patch.
- Do not loosen golden tests to hide decoder behavior changes.
- Do not implement platform-specific hacks in `crates/core` unless the behavior is truly shared.

## Verification

For documentation-only edits:

```bash
cargo check -p khmerime_windows_tsf
```

For scaffold/contract Rust changes:

```bash
cargo test -p khmerime_windows_tsf
```

For changes that touch shared session behavior:

```bash
cargo test -p khmerime_session
cargo test -p khmerime_windows_tsf
```

For changes that affect ranking, segmentation, or decoder output:

```bash
make test-golden
make test
```

For final manual Windows validation, use the smoke checklist above on an actual Windows machine or VM.
