# Linux IBus NIDA Direct Mode

## Status

Planned for Linux IBus phase 1.

## Problem

KhmerIME's roman decoder cannot produce every possible Khmer word, especially
names, places, and rare technical terms that are missing from the lexicon. Users
need a deterministic manual fallback that stays inside KhmerIME instead of
requiring them to leave the IME or install a separate keyboard.

## Goals

- Add a shared input mode model with `roman` and `nida`.
- Keep `roman` as the existing decoder-driven KhmerIME behavior.
- Add `nida` as a direct Khmer keymap mode that bypasses decoder suggestions.
- Start with Linux IBus while keeping the session contract reusable by future
  Windows, macOS, Android, and iOS adapters.
- Expose both `KhmerIME` and `KhmerIME NIDA` IBus engines:
  - `KhmerIME` starts in `roman`.
  - `KhmerIME NIDA` starts in `nida`.
- Let CapsLock toggle `roman` and `nida` when Linux IBus delivers the key event.
- Provide an IBus property that displays the current mode and toggles the same
  shared session state as CapsLock.

## Non-Goals

- Do not replace the existing roman decoder path.
- Do not implement every platform in phase 1.
- Do not add automatic learning or user-dictionary capture from NIDA input.
- Do not duplicate decoder or keymap logic inside the Python adapter.
- Do not invent NIDA mappings without an auditable reference table.

## Ownership

- `crates/session/src/ime_session.rs` owns the input mode state, mode-switch
  commands, snapshot field, and shared key semantics.
- A dedicated keymap module owns NIDA direct lookup from normalized key identity
  to Khmer output.
- `adapters/linux-ibus/src/bin/khmerime_ibus_bridge.rs` owns the JSON bridge
  commands for initial mode and mode toggling.
- `adapters/linux-ibus/python/khmerime_ibus_engine.py` owns IBus engine
  registration, IBus property rendering, and platform event forwarding.

## Session Contract

`SessionSnapshot` must include the active input mode as a stable serialized
value:

- `roman`
- `nida`

Session commands must include:

- set input mode
- toggle input mode

Changing input mode must clear active composition, candidates, segmented state,
and pending refinement state. The session is the source of truth; adapters must
not maintain a separate mode state.

## NIDA Mode Behavior

In `nida` mode:

- mapped printable keys immediately commit their Khmer output
- unmapped printable keys pass through to the application
- Ctrl, Alt, Super, Meta, and Hyper shortcuts pass through
- navigation and editing keys pass through
- no roman preedit is created
- decoder candidates are not shown

CapsLock toggles mode only when the platform delivers it to KhmerIME. If a Linux
desktop intercepts CapsLock before IBus, the IBus property remains the reliable
mode switch.

## Linux IBus Behavior

The Linux adapter should register two engines that share the same bridge/session
implementation:

- `khmerime` starts with `roman`
- `khmerime-nida` starts with `nida`

Both engines expose the same mode property. Clicking the property sends the
shared toggle command to the session, then renders the updated snapshot.

## Keymap Data Contract

The Linux keymap must live in an auditable data file such as:

`data/keymaps/nida_linux.csv`

The file should record enough information to review each mapping, including:

- base key identity
- required modifier state
- emitted Khmer text
- source/reference note if the row comes from a published NIDA table

The first reference candidate is the Microsoft `KBDKNI.DLL` Khmer (NIDA)
layout as exposed by `kbdlayout.info`. It lists the layout as "Khmer (NIDA)"
with file description "Khmer (NIDA) Keyboard Layout" and internal name
`kbdkni`.

Linux NIDA lookup should prefer physical key identity plus modifier state over
printable `keyval`. For example, shifted symbol keyvals differ between US,
Japanese, and other layouts, but a NIDA layout is defined by the physical key
position. `keyval` remains useful for pass-through and diagnostics, not as the
only lookup key for mapped NIDA output.

Platform adapters may eventually use platform-specific NIDA-compatible keymaps
when native keyboard APIs expose different key identities.

## Verification

Required tests for phase 1:

- session defaults to `roman`
- session can start in `nida`
- set/toggle mode updates `SessionSnapshot.input_mode`
- switching mode clears active composition and candidates
- NIDA mapped printable keys commit Khmer directly
- NIDA unmapped printable keys pass through
- shortcuts and navigation keys pass through
- CapsLock toggles mode when delivered
- Linux bridge can start in either initial mode
- Linux bridge/property toggle updates snapshot mode
- IBus protocol tests cover `KhmerIME` and `KhmerIME NIDA` start modes

Run:

- `cargo fmt --all`
- `cargo test -p khmerime_session`
- `cargo test -p khmerime_linux_ibus --test ibus_bridge_protocol`
- `make ibus-smoke`
