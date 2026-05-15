# Linux IBus Startup Performance

## Status

Implemented for Linux IBus phase 1 performance work.

Follow-up manual validation should still be done in real Ubuntu IBus clients
such as gedit because some focus/reset behavior is application-driven.

## Problem

Switching to KhmerIME through Linux IBus can feel slow on first activation. The
historical bridge startup path built the live transliterator and Hybrid commit
refiner before the Python IBus adapter received its first snapshot. That meant
the user could select KhmerIME, but the engine might not feel responsive until
the Rust bridge finished full startup work.

The same class of issue was previously addressed in the Web path by making a
lighter phase-A engine usable before full decoder/data readiness. Linux IBus
should use the same product principle: typing must become usable quickly, while
full suggestion quality can arrive shortly after.

## Goals

- Make Linux IBus first activation responsive quickly.
- Keep Python IBus callback handling synchronous for this first fix.
- Move full engine construction out of the blocking startup path.
- Provide basic roman preedit and dictionary candidates during phase-A.
- Restore full live behavior after background warmup.
- Preserve long-phrase quality when possible without making typing feel stuck.
- Add basic logs for phase-A readiness, full warmup, full upgrade, and Enter
  wait timeout.

## Non-Goals

- Do not rewrite the Python bridge client as fully asynchronous in this phase.
- Do not add visible loading text or candidate status rows in this phase.
- Do not change Dioxus Web behavior.
- Do not change Windows TSF behavior yet.
- Do not run full Hybrid decoding on every keypress.
- Do not make startup wait for full Hybrid commit refinement before the first
  usable snapshot.

## Current Surface

Files involved:

- `adapters/linux-ibus/python/khmerime_ibus_engine.py`
- `adapters/linux-ibus/python/ibus_bridge_client.py`
- `adapters/linux-ibus/python/ibus_refinement.py`
- `adapters/linux-ibus/src/bin/khmerime_ibus_bridge.rs`
- `adapters/linux-ibus/src/lib.rs`
- `adapters/linux-ibus/tests/ibus_bridge_protocol.rs`
- `crates/session/src/ime_session.rs`
- `crates/core/src/roman_lookup/transliterator.rs`

Current behavior after this phase:

- Python creates `BridgeClient` during `KhmerIMEEngine.__init__`.
- `BridgeClient` starts `khmerime_ibus_bridge`.
- Python immediately asks the bridge for `snapshot`.
- The bridge answers the initial snapshot from a phase-A session.
- Phase-A startup builds:
  - phase-A transliterator from embedded compiled data;
  - desktop history;
  - an `ImeSession` with segmented preview disabled.
- Background full warmup builds:
  - live transliterator with `DecoderConfig::shadow_interactive()`;
  - Hybrid commit refiner.
- If full warmup completes while composition is active, the bridge attaches the
  commit refiner and defers full live transliterator replacement until the
  composition is idle.
- IBus visible long-composition refinement is debounced and stale checked in
  Python.
- Python ignores key-release events before debounce cancellation so release
  callbacks do not cancel the pending refinement timer.
- Python does not clear visible preedit before the Enter bridge response. It
  clears preedit only after a non-empty `commit_text`, and preserves active
  preedit if the bridge returns an empty commit with an empty snapshot.

## Desired Startup Model

```text
IBus activates KhmerIME
        |
        v
Python adapter starts bridge
        |
        v
Rust bridge builds phase-A session quickly
        |
        v
Python receives first snapshot
        |
        v
User can type with basic candidates
        |
        v
Rust bridge warms full engine in background
        |
        v
Full live session + commit refiner become ready
        |
        v
Bridge upgrades when composition is idle
```

## Phase-A Behavior

Phase-A must provide:

- roman raw preedit;
- basic legacy-style dictionary candidates;
- normal candidate cycling and selection for the basic candidate list;
- no segmented preview;
- no required Hybrid commit refiner before typing works.

Phase-A must not:

- call `shadow_observation` on every printable key;
- build segmented preview state;
- require khPOS/next-word/full ranking resources before first snapshot;
- block the first snapshot on the Hybrid commit refiner.

## Full Warmup Behavior

Background warmup starts immediately after the bridge creates the phase-A
session.

The background warmup should build:

- full live transliterator using `DecoderConfig::shadow_interactive()`;
- full commit refiner using Hybrid mode;
- any full resources required by those transliterators.

When warmup finishes:

- if the current session has empty composition, upgrade immediately;
- if the user is composing, mark full mode as pending;
- apply the full upgrade after commit, reset, focus out, or backspace-to-empty;
- preserve input mode and cursor location across upgrade;
- preserve history.

## Enter Behavior During Phase-A

If the user presses Enter during phase-A:

1. If the full commit refiner is ready, use the normal refined commit path.
2. If full warmup is still running, wait up to `500 ms` for the full refiner.
3. If the refiner becomes available within the timeout, use it.
4. If the timeout expires, commit the current phase-A selected/basic candidate
   or raw fallback.

The Enter path must never wait without a timeout.

## Visible Refinement During Phase-A

Visible long-phrase refinement may run during phase-A only after the full
refiner is available.

Rules:

- keep the existing debounce behavior;
- apply only if `raw_preedit` still matches;
- do not override explicit user selection;
- do not require full live session swap;
- do not enable segmented preview during phase-A.

## Session Contract Changes

Add an explicit session option for segmented preview behavior.

Expected shape:

```text
SegmentedPreviewMode::Disabled
SegmentedPreviewMode::Enabled
```

Phase-A sessions use `Disabled`.

Full sessions use `Enabled`.

When disabled, `ImeSession::recompute_composition_state()` must skip
`shadow_observation` and `build_segmented_session`. Hiding segment preview in the
bridge is not enough because the expensive work would still happen.

## Transliterator Construction

Add a native embedded phase-A constructor.

Expected shape:

```rust
Transliterator::from_default_phase_a_data(config)
```

The native phase-A constructor should reuse the existing phase-A path by parsing
the embedded compiled lexicon and skipping khPOS, next-word stats, full ranking
structures, fuzzy search index, and composer construction where the existing
phase-A implementation already skips them.

Linux IBus phase-A should use this constructor with basic legacy behavior.

## Bridge Ownership

The Rust bridge owns readiness and warmup.

Python should remain mostly a renderer and IBus callback adapter. It should not
own engine lifecycle decisions or decoder internals.

The bridge should expose readiness in snapshots or responses with stable values
such as:

```text
phase_a
full_pending
full
failed
```

Exact serialized names can be adjusted during implementation, but protocol tests
must lock the final names.

## Python Behavior

For this phase:

- keep `BridgeClient.call(...)` synchronous;
- keep key-event handling synchronous;
- render snapshots as today;
- do not add visible loading UI;
- log readiness/roundtrip information where useful.
- key-release events must not cancel pending refinement debounce work;
- Enter must not clear preedit before a non-empty bridge commit response is
  returned.

Python timeout/fallback behavior is deferred unless measurements show that
bridge calls remain slow after phase-A startup.

## Logging

Add basic logs for:

- bridge process start;
- phase-A session start/end;
- full warmup start/end/failure;
- full upgrade applied;
- full upgrade deferred because composition is active;
- Enter waited for full refiner;
- Enter full-refiner wait timed out;
- Enter preserved active preedit because the bridge returned an empty commit;
- refinement applied/stale/retry decisions;
- first snapshot response;
- first key-event roundtrip if practical.

The goal is debugging and local validation, not a full telemetry framework.

## Acceptance Criteria

- Linux IBus bridge can answer the first `snapshot` after phase-A construction
  without waiting for the full Hybrid commit refiner.
- During phase-A, typing roman input shows raw preedit and basic candidates.
- During phase-A, segmented preview is not computed or shown.
- Background full warmup starts immediately after phase-A startup.
- Full live transliterator and Hybrid commit refiner are installed after
  warmup, but only when composition is idle.
- If warmup completes during active composition, full upgrade is deferred until
  the composition becomes idle.
- Enter during phase-A waits at most `500 ms` for a full refiner, then falls back
  safely.
- Debounced visible refinement can use the full refiner once ready without
  forcing full live session swap.
- Key-release events do not cancel a pending visible-refinement debounce.
- Enter does not blank active preedit before receiving a non-empty commit
  response.
- Explicit candidate selection is not overridden by refinement or upgrade.
- Python IBus code remains synchronous in this phase.
- Basic startup/warmup/timeout logs are written for debugging.

## Validation

Required automated checks:

- `cargo fmt --all`
- `cargo test -p khmerime_session`
- `cargo test -p khmerime_linux_ibus --test ibus_bridge_protocol`
- `.venv/bin/pytest tests/test_ibus_mode_property.py`
- `make ibus-smoke`

Recommended protocol tests:

- bridge initial snapshot reports phase-A readiness quickly;
- phase-A input returns candidates but no segmented preview;
- full warmup readiness eventually reports full or pending-full;
- full upgrade is deferred while `raw_preedit` is non-empty;
- full upgrade applies after commit/reset/empty composition;
- Enter during phase-A uses full refiner if it becomes available within timeout;
- Enter during phase-A falls back if refiner is unavailable after timeout;
- visible refinement ignores stale raw input;
- key-release events do not cancel pending debounced refinement;
- Enter with active preedit does not clear UI before non-empty commit;
- explicit non-default selection remains preserved.

Manual Ubuntu IBus checks:

- switch to KhmerIME and immediately type a short word;
- verify preedit and candidates appear without a multi-second wait;
- type a long phrase and pause after full warmup; verify refined candidate can
  appear;
- type a long phrase immediately after switching and press Enter; verify Enter
  does not hang beyond the timeout;
- type a long phrase immediately after switching and press Enter; verify the
  preedit does not become blank without committed text;
- verify segmented preview appears after full readiness for new compositions.

## Open Questions

- What exact latency budget should phase-A first snapshot target?
- Should full warmup errors leave the bridge permanently in phase-A or retry
  later?
- Should the future Python timeout/fallback layer use per-command timeouts or
  only protect first snapshot and Enter?
- Should focus/reset during active preedit commit, preserve, or cancel
  composition in specific IBus clients?
