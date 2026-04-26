# Ubuntu IBus Native Keyboard Guide

This document explains how the Ubuntu keyboard path works in `khmerime`, from the desktop input-source switch down to the Rust transliteration engine.

The important idea is: on Ubuntu we are not building a physical keyboard layout. We are building an input method engine, or IME. Ubuntu uses IBus to load that engine, send key events to it, show preedit/candidate UI, and commit the selected Khmer text into the focused application.

## Short Version

When the user enables KhmerIME in Ubuntu and types `jea`:

```text
Keyboard key press
  -> Ubuntu / GTK / IBus
  -> Python IBus engine adapter
  -> Rust JSON-line bridge
  -> Rust ImeSession
  -> Rust Transliterator
  -> suggestions / preedit / commit result
  -> Python adapter updates IBus UI
  -> Ubuntu commits Khmer text into the focused app
```

So the Ubuntu keyboard is made from five pieces:

1. IBus component registration so Ubuntu can discover the engine.
2. A Python IBus engine process because IBus exposes its desktop callbacks through Python/GObject APIs.
3. A Rust bridge process that speaks a small JSON-line protocol.
4. A platform-neutral `ImeSession` that owns composition, candidate selection, and commit behavior.
5. The existing `Transliterator` engine that turns roman input into ranked Khmer suggestions.

## Why IBus?

Ubuntu GNOME commonly uses IBus for input methods. A normal keyboard layout maps one key to one character, but `khmerime` needs more than that:

- the user types roman text such as `khnhomttov`,
- the engine keeps temporary composition text before final commit,
- the engine shows candidates,
- the user can cycle or choose candidates,
- the engine can segment a long roman phrase into chunks,
- the chosen result can be learned into history.

That behavior is IME behavior, not static keyboard-layout behavior. IBus gives us the native desktop hooks for it.

## What Ubuntu Sees

Ubuntu only needs to discover an IBus component XML file. The local install script writes this file to the system IBus component directory:

```text
/usr/share/ibus/component/khmerime.xml
```

That XML says roughly:

```text
name:        org.freedesktop.IBus.KhmerIME
engine:      khmerime
long name:   AngkorIME
language:    km
layout:      us
symbol:      ខ
exec:        /usr/libexec/khmerime/khmerime-ibus-engine --ibus --bridge-path /usr/libexec/khmerime/khmerime-ibus-bridge
```

After IBus refreshes its cache, Ubuntu Settings can show this as an input source. In Settings, search for `Khmer`, then add `KhmerIME`.

The user switches to it with the desktop input-source shortcut, usually `Super+Space` on GNOME. That switch is the v1 on/off model. We are not adding a separate in-app toggle for the native keyboard path.

## Installed Files

`make ibus-install` runs `scripts/platforms/linux/ibus/install_engine.sh`.

It builds and installs:

```text
target/release/khmerime_ibus_bridge
  -> /usr/libexec/khmerime/khmerime-ibus-bridge

adapters/linux-ibus/python/khmerime_ibus_engine.py
  -> /usr/libexec/khmerime/khmerime-ibus-engine

adapters/linux-ibus/python/ibus_segment_preview.py
  -> /usr/libexec/khmerime/ibus_segment_preview.py

component XML
  -> /usr/share/ibus/component/khmerime.xml
```

The script may ask for `sudo` because Ubuntu IBus component discovery normally reads system directories.

Remove those files with:

```bash
make ibus-uninstall
```

## Runtime Pieces

### 1. `khmerime.xml`

Owned by:

```text
scripts/platforms/linux/ibus/install_engine.sh
```

Purpose:

- tells IBus the engine exists,
- tells IBus which command starts it,
- assigns the language `km`, symbol `ខ`, and engine name `khmerime`,
- makes the engine appear in Ubuntu input-source settings.

This is the discovery layer only. It does not implement typing behavior.

### 2. Python IBus Adapter

Owned by:

```text
adapters/linux-ibus/python/khmerime_ibus_engine.py
```

Purpose:

- starts as the process launched by IBus,
- receives IBus callbacks such as key events, focus changes, reset, enable, disable, and cursor movement,
- sends those callbacks to the Rust bridge as JSON commands,
- receives snapshots from Rust,
- updates native IBus UI: preedit text, lookup candidates, auxiliary segment preview,
- commits final text back to the focused app through IBus.

This file is Python because IBus desktop integration is exposed through GObject introspection (`gi.repository.IBus`). The actual IME logic stays in Rust.

### 3. Rust Bridge

Owned by:

```text
adapters/linux-ibus/src/bin/khmerime_ibus_bridge.rs
```

Purpose:

- starts the Rust transliteration engine,
- owns one `ImeSession`,
- reads one JSON command per line from stdin,
- writes one JSON response per line to stdout,
- saves desktop history after commits.

The bridge keeps the Python adapter small. Python handles desktop callbacks; Rust handles IME behavior.

### 4. Session State Machine

Owned by:

```text
crates/session/src/ime_session.rs
```

Purpose:

- stores whether the IME is enabled and focused,
- stores the current roman composition text,
- stores visible candidates and selected candidate index,
- stores segmented phrase state when a long token can be split into chunks,
- defines key behavior for native IME flow,
- produces a snapshot that adapters can render.

This layer is intentionally platform-neutral. It should not know about IBus, GTK, GNOME, or Python.

### 5. Core Transliterator

Owned mainly by:

```text
crates/core/src/roman_lookup/
crates/core/src/composer/
crates/core/src/decoder/
```

Purpose:

- normalizes roman input,
- searches the lexicon,
- segments long roman strings,
- ranks Khmer candidates,
- learns from committed choices.

This is the same core engine used by the web app, desktop app, CLI, and native IBus path.

## Data Flow For One Key Press

Example: the user has switched to KhmerIME and presses `j`.

```text
1. The focused app receives a key event from the desktop.
2. IBus routes the key event to KhmerIME because it is the active input source.
3. `khmerime_ibus_engine.py` receives `do_process_key_event(keyval, keycode, state)`.
4. Python sends this JSON line to the bridge:

   {"cmd":"process_key_event","keyval":106,"keycode":0,"state":0}

5. `khmerime_ibus_bridge` parses the command.
6. The bridge calls `ImeSession::process_key_event(...)`.
7. `ImeSession` appends `j` to the roman preedit.
8. `ImeSession` asks `Transliterator` for candidates.
9. The bridge returns a JSON response containing:
   - whether the key was consumed,
   - optional committed text,
   - current preedit,
   - candidates,
   - selected candidate index,
   - segment preview if active.
10. Python updates IBus preedit and lookup-table UI.
11. The focused app does not receive raw `j` if the key was consumed.
```

The same loop repeats for `e`, `a`, arrows, `Space`, `Enter`, and candidate number keys.

## Preedit, Candidates, And Commit

Three IME terms matter here.

`preedit` is temporary text.

For `jea`, the preedit may show the roman text or the currently composed phrase before it is committed. The focused application has not permanently received it yet.

`candidates` are possible outputs.

For a roman token, the Rust engine returns ranked Khmer suggestions. The Python adapter shows them in the IBus lookup table.

`commit` is the final insertion.

When the user presses `Enter`, the selected candidate or composed phrase is sent to IBus as committed text. IBus inserts it into the focused application.

## Key Behavior

### Printable ASCII

Printable ASCII keys update the roman preedit and recompute suggestions.

Example:

```text
j -> preedit "j"
e -> preedit "je"
a -> preedit "jea"
```

The session then asks the transliterator for candidates after each update.

### Candidate Selection

When a normal single-token candidate list is active:

```text
Up / Down   cycle candidates
Space       cycle candidates
1..9        choose candidate number, but do not immediately commit
Enter       commit selected candidate
Backspace   edit roman preedit and recompute candidates
Esc         cancel the current session
```

When no candidate UI is active, `Up` and `Down` pass through to the host app instead of being stolen by the IME.

### Segmented Phrase Mode

For long roman input, the engine may split the text into segments.

Example:

```text
khnhomttov
  -> khnhom + ttov
  -> ខ្ញុំ + ទៅ
```

In segmented mode:

```text
Left / Right  move the focused segment
Up / Down     cycle candidates in the focused segment
Space         cycle candidates in the focused segment
1..9          choose a candidate for the focused segment, but do not commit yet
Enter         commit the full composed phrase
Backspace     edit roman preedit and rebuild segmentation
Esc           cancel the current session
```

The standard IBus lookup table shows candidates for the focused segment. The IBus auxiliary text shows a compact segment preview so the user can see which chunk is being edited.

## Snapshot Contract

The Rust session returns a snapshot after each command. The Python adapter renders this snapshot.

Important fields include:

```text
enabled                 whether the IME is enabled
focused                 whether an app currently has IME focus
preedit                 visible composition text
raw_preedit             original roman composition text
candidates              current candidate strings
candidate_display       candidate metadata for labels and hints
selected_index          selected candidate index
segmented_active        whether phrase segmentation is active
focused_segment_index   which segment is being edited
segment_preview         compact per-segment preview data
cursor_location         current app cursor rectangle
```

This contract is why the split works: Rust owns state and decisions; Python only renders and forwards desktop events.

## History Persistence

Desktop history is stored under:

```text
~/.config/khmerime/history.tsv
```

Flow:

```text
user commits candidate
  -> ImeSession records learning
  -> bridge sees `history_changed: true`
  -> bridge saves history through DesktopHistoryStore
```

That history can influence future ranking.

## How To Install And Try It

Run:

```bash
make ibus-install
make ibus-smoke
```

Then in Ubuntu GNOME:

1. Open `Settings`.
2. Go to `Keyboard`.
3. Under `Input Sources`, add a new source.
4. Search for `Khmer`.
5. Select `KhmerIME`.
6. Switch input sources with `Super+Space`.
7. Open a text editor.
8. Type a roman query such as `jea`.
9. Use `Space`, arrow keys, number keys, or `Enter` to test candidate behavior.

If the source does not appear, try:

```bash
ibus write-cache
ibus restart
```

If it still does not appear, log out and log back in. IBus discovery can be session-cache sensitive.

## How To Smoke Test Without The Desktop UI

Run:

```bash
make ibus-smoke
```

This verifies two things:

```text
bridge protocol check
  Sends JSON commands for `j`, `e`, `a`, and `Enter` to the bridge and expects committed text.

IBus discovery check
  Starts a temporary DBus/IBus session and checks whether `khmerime` appears in `ibus list-engine`.
```

The discovery check may be skipped in restricted environments where a nested DBus/IBus session cannot run.

## Debugging

### Check Whether IBus Knows About The Engine

```bash
ibus list-engine | grep khmerime
```

Expected: a line mentioning `khmerime`.

### Check The Adapter Log

The Python adapter writes logs to:

```text
~/.cache/khmerime/ibus_engine.log
```

You can override the path with:

```bash
KHMERIME_IBUS_LOG=/tmp/khmerime-ibus.log make ibus-install
```

For a running installed engine, set the environment variable in the session that launches IBus if you need a custom log path.

### Check The Bridge Directly

Build the bridge:

```bash
cargo build --release --bin khmerime_ibus_bridge
```

Send commands manually:

```bash
printf '%s\n' \
  '{"cmd":"focus_in"}' \
  '{"cmd":"process_key_event","keyval":106,"keycode":0,"state":0}' \
  '{"cmd":"process_key_event","keyval":101,"keycode":0,"state":0}' \
  '{"cmd":"process_key_event","keyval":97,"keycode":0,"state":0}' \
  '{"cmd":"process_key_event","keyval":65293,"keycode":0,"state":0}' \
  '{"cmd":"shutdown"}' \
  | target/release/khmerime_ibus_bridge
```

This should produce JSON responses. The Enter response should include a non-empty `commit_text` if the engine found or accepted a commit.

## What To Change When Adding Features

Use the smallest owning subsystem.

```text
Change candidate ranking or roman lookup
  -> crates/core/src/roman_lookup/

Change long-token segmentation
  -> crates/core/src/composer/

Change decoder mode behavior
  -> crates/core/src/decoder/

Change native key behavior, preedit, commit, or segmented selection semantics
  -> crates/session/src/ime_session.rs

Change JSON protocol or history save timing
  -> adapters/linux-ibus/src/bin/khmerime_ibus_bridge.rs

Change IBus callback wiring, lookup table rendering, preedit rendering, or auxiliary preview UI
  -> adapters/linux-ibus/python/khmerime_ibus_engine.py

Change install paths or component registration
  -> scripts/platforms/linux/ibus/install_engine.sh and scripts/platforms/linux/ibus/uninstall_engine.sh
```

Do not put IBus-specific logic into `crates/core`. The core engine should stay reusable by the web app, desktop app, CLI, and future platform adapters.

## Verification Matrix

For documentation-only changes, no runtime test is required.

For native keyboard behavior changes, run the focused checks:

```bash
cargo test -p khmerime_session
cargo test -p khmerime_linux_ibus --test ibus_bridge_protocol
bash scripts/platforms/linux/ibus/smoke_test.sh
```

For changes that affect decoder output, also run:

```bash
cargo test
cargo test --test decoder_golden
```

For final manual confidence on Ubuntu:

1. Run `make ibus-install`.
2. Add `KhmerIME` in Settings if needed.
3. Switch to it with `Super+Space`.
4. Type a short token like `jea` and commit it.
5. Type a long token like `khnhomttov`.
6. Move between segments with `Left` and `Right`.
7. Cycle segment candidates with `Space`, `Up`, or `Down`.
8. Commit the full phrase with `Enter`.

## Current Scope And Limitations

The current Ubuntu path is a developer-local v1 integration.

It does not yet provide:

```text
Debian package
Ubuntu PPA
GNOME extension
custom settings UI
per-app configuration
system-wide installer UX
```

Those can come later. The important maintained boundary today is the IBus engine path: system source switching, native preedit/candidate UI, Rust session logic, and the shared transliteration engine.
