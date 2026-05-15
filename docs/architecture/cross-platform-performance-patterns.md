# KhmerIME Cross-Platform Performance Patterns

## Purpose

This document records repeatable performance and UX patterns that can make
KhmerIME feel slow when a user first activates the IME or starts typing. The
immediate motivation is Linux IBus first-activation latency, but the same class
of issue can repeat in Web, Windows TSF, and future mobile adapters.

The goal is to document the architecture pattern before coding another
platform-specific fix:

- keep platform activation lightweight;
- warm the engine before the first key event where possible;
- avoid heavy decoder/data work on active typing paths;
- provide fallback behavior when the full engine is not ready;
- measure startup and first-suggestion latency.

This document separates confirmed findings from source/git inspection and
implementation guidance for future work.

## Confirmed Findings From Source Inspection

| Area | Confirmed finding |
| --- | --- |
| Core data loading | `crates/core/src/roman_lookup/transliterator.rs` loads compiled lexicon, khPOS stats, and next-word stats in `Transliterator::from_default_data_with_config`. |
| Phase-A path | `Transliterator::from_phase_a_bytes` builds a lighter transliterator from lexicon bytes and intentionally skips heavier fuzzy/ranking/composer structures. |
| Web readiness | `apps/dioxus-app/src/engine_registry.rs` has `EngineReadiness::{Booting, LegacyReady, FullReady, Failed}` and uses separate phase-A and full transliterators for wasm `fetch-data` builds. |
| Web startup | `apps/dioxus-app/src/startup_fetch.rs` fetches `roman_lookup.lexicon.bin`, `khpos.stats.bin`, and `next_word.stats.bin`; it can expose phase-A readiness before full data promotion. |
| Web shadow gating | `apps/dioxus-app/src/ui/editor/candidate_pipeline.rs` only schedules shadow refinement when the engine is full-ready. Shadow refinement is debounced and checked for stale requests. |
| Non-wasm Dioxus | Non-wasm Dioxus builds keep engines in `OnceLock`s and perform a warmup by calling legacy suggestions during startup bootstrap. |
| Linux IBus bridge startup | `adapters/linux-ibus/src/bin/khmerime_ibus_bridge.rs` now creates a phase-A session first, reports bridge readiness, and warms the full live transliterator plus Hybrid commit refiner in the background. |
| Linux IBus Python startup | `adapters/linux-ibus/python/khmerime_ibus_engine.py` creates `BridgeClient` in engine initialization and immediately requests a snapshot from the Rust bridge. |
| Linux IBus key path | `do_process_key_event` calls the Rust bridge synchronously and then updates IBus preedit/candidates from the returned snapshot. |
| Linux IBus refinement | Long-composition visible refinement is debounced in `adapters/linux-ibus/python/ibus_refinement.py` and runs in a background thread before applying the response on the GLib idle path. |
| Linux IBus event hygiene | Key-release events must not cancel pending refinement debounce work, and Enter must not clear visible preedit before the bridge returns a non-empty commit. |
| Native session recompute | `crates/session/src/ime_session.rs` recomputes suggestions and segmented state synchronously after printable input. |

## Confirmed Findings From Git History

Commit `4fff1ea0af791b06a4b4bb1732bd00cc4999857d`
(`wip(web): phase startup for iOS + decoder readiness gating`) documents the
previous Web startup mitigation.

Confirmed from the commit message and diff:

- added `EngineReadiness` states;
- split wasm startup into phase-A "legacy-ready" and phase-B "full-ready";
- added startup profiling/tracing query profiles:
  `defer_full`, `baseline`, `lexicon_only`, and
  `baseline_compression_audit`;
- loaded lexicon first;
- deferred khPOS/full promotion;
- gated shadow work until the full engine was ready;
- added a phase-A transliterator path;
- added build-time khPOS trimming knobs.

The poor UX pattern was not only "large files exist". It was that first usable
typing could be delayed by full engine/data readiness. The mitigation was to
make a smaller useful engine available first, then promote to the full engine in
the background.

## Slow Startup Pattern

The repeatable failure mode is:

```text
Platform activation
        |
        v
Adapter starts
        |
        v
Engine/data/decoder initialization happens synchronously
        |
        v
First key event waits, or first suggestions wait
        |
        v
User sees "KhmerIME is selected" but typing feels broken or delayed
```

This can happen when:

- large data files are loaded during focus or first key handling;
- search indexes are built on the first suggestion request;
- a full decoder is initialized before any fallback suggestions can be shown;
- shadow/experimental decoders run during active typing;
- a platform adapter calls the core engine synchronously without timeouts,
  readiness state, or fallback behavior.

## Web UI Case Study

The Web path already contains the clearest mitigation pattern.

Current relevant design:

```text
wasm fetch-data startup
        |
        +-- fetch roman_lookup.lexicon.bin
        |
        +-- build phase-A transliterator
        |
        +-- mark EngineReadiness::LegacyReady
        |
        +-- continue fetching khPOS + next-word data
        |
        +-- promote full transliterator
        |
        +-- mark EngineReadiness::FullReady
```

Key lessons:

- Phase-A only needs to be good enough to unlock typing.
- Full ranking and expensive supporting data can arrive later.
- Shadow/full refinement should be gated until full readiness.
- Startup diagnostics need named stages, not only total page load time.
- If full engine startup fails, the UI should still have a clear readiness/error
  state instead of silently blocking the typing path.

The current Web implementation uses this model through
`engine_registry.rs`, `startup_fetch.rs`, and candidate-pipeline gating.

## Linux IBus Risk Analysis

The Linux IBus path is architecturally different from the Web path:

```text
IBus activates KhmerIME engine
        |
        v
Python adapter __init__
        |
        v
BridgeClient starts khmerime_ibus_bridge process
        |
        v
Rust bridge builds phase-A session
        |
        +-- load embedded phase-A lexicon path
        +-- load desktop history
        +-- return readiness=phase_a snapshot
        |
        v
Python adapter can render preedit/candidates
        |
        v
Rust bridge warms full engines in background
        |
        +-- full live shadow_interactive transliterator
        +-- Hybrid commit refiner
        |
        v
Bridge applies full upgrade when composition is idle
```

Confirmed current behavior:

- Bridge process startup is synchronous from the Python adapter's perspective.
- The Rust bridge answers the first snapshot from a phase-A session instead of
  waiting for full Hybrid commit-refiner construction.
- Bridge responses expose readiness values:
  `phase_a`, `full_pending`, `full`, and `failed`.
- If full warmup completes while the user is composing, the bridge attaches the
  commit refiner but defers full live-session replacement until composition is
  idle.
- The native session recomputes suggestions and segmented state synchronously
  after each printable key.
- Phase-A sessions disable segmented preview so printable key handling avoids
  `shadow_observation` and segmented-session construction.
- Full sessions use `DecoderConfig::shadow_interactive` for live behavior and a
  Hybrid commit refiner for commit/refinement quality.

Existing mitigations:

- The bridge uses a phase-A/full-pending/full readiness model.
- Full warmup runs outside the initial snapshot path.
- Full live upgrade is deferred while `raw_preedit` is active.
- IBus long-composition refinement is debounced and stale-checked.
- Expensive full Hybrid refinement is used as commit/refinement support rather
  than as the only live candidate source.
- Key-release events are ignored before refinement cancellation so release
  callbacks do not kill the idle debounce timer.
- Enter clears visible preedit only after a non-empty commit response is
  returned. If the bridge returns an empty commit with an empty snapshot while
  preedit was active, the adapter preserves the existing preedit instead of
  blanking the text.

Open Linux risk:

- The bridge client call path is still synchronous and has no general
  per-command timeout/fallback layer.
- First snapshot is faster than full startup, but phase-A construction still
  parses embedded data and should keep a latency budget.
- Focus/reset behavior can still clear active preedit when applications move
  focus or reset the input context; that should be reviewed separately from
  startup readiness.

## Cross-Platform Repeatability

These patterns can repeat on every platform:

| Pattern | Why it hurts UX | Example risk surface |
| --- | --- | --- |
| Load large dictionary/POS data on first focus | User selects KhmerIME but cannot type smoothly yet. | Linux IBus bridge startup, Windows TSF activation, mobile keyboard first open. |
| Initialize heavy decoder on first key | First typed character appears delayed or is dropped by platform timing. | Any platform adapter calling `suggest` before warmup. |
| Run shadow/experimental decoder on hot path | Suggestions lag during fast typing. | Web shadow mode, native segmented preview, long phrase refinement. |
| Parse CSV/text data at runtime | Runtime startup depends on slow parsing and allocation. | Future external data update loader if it bypasses compiled blobs. |
| Build search/ranking indexes lazily | The first query pays the full indexing cost. | Any `OnceLock` initialized from a key event. |
| Block UI/input thread | Platform considers IME unresponsive. | IBus GLib callback, Windows TSF edit session, browser main thread, mobile IME service. |
| Missing readiness/fallback state | User sees no clear reason suggestions are absent. | Any adapter without "booting/ready/failed" state. |
| Missing stale-request checks | Old expensive result overwrites newer typing state. | Background decode/refinement. |
| Release events cancel idle work | Platform key-release callbacks cancel a pending debounce before it fires. | IBus debounced long-phrase refinement. |
| Pre-clearing preedit before commit | UI removes composition before the engine confirms a non-empty commit. | IBus Enter handling during startup/full handoff. |
| No latency metrics | Fixes become guesswork and regressions are easy. | Cross-platform startup and suggestion paths. |

## Repeatable Fixing Patterns

Use the same shape across platforms:

```text
Platform Activation
        |
        v
Lightweight Adapter Ready
        |
        v
Background Engine Warmup
        |
        +-- load dictionary/data
        +-- initialize decoder
        +-- prepare search index
        +-- prepare suggestion cache
        |
        v
Engine Ready

First Key Event
        |
        +-- if engine ready: normal suggestions
        +-- if engine not ready: non-blocking fallback
```

Recommended behavior:

| Lifecycle point | Do | Avoid |
| --- | --- | --- |
| App/process startup | Start lightweight platform shell, register callbacks, begin warmup. | Blocking until every decoder and language resource is ready. |
| First activation/focus | Ensure adapter can accept events and show a clear state. | Starting heavy engine construction synchronously from focus callback. |
| Background warmup | Build full transliterator, load stats, build search/ranking structures. | Touching UI state after stale focus/session changes. |
| First key event | Return quickly with raw preedit, basic candidate, or phase-A suggestions. | Blocking for full Hybrid/shadow decode. |
| Active typing | Use bounded live decoder work and stale-request guards. | Running unbounded full phrase refinement on every keystroke. |
| Idle/pause | Run debounced refinement if the request is still current. | Applying old refinement after the user typed more characters. |
| Commit | Use commit-time fallback/refinement only when bounded and explicit selection is preserved. | Overriding a user's selected candidate. |

## Cross-Platform Developer Guidance

### Web

- Keep the phase-A/phase-B model.
- Keep startup profiles for diagnostics.
- Keep shadow/full decoder work gated by `EngineReadiness::FullReady`.
- Do not make first usable typing depend on khPOS or next-word data.

### Linux IBus

- Keep IBus callback handling lightweight.
- Keep the bridge readiness state explicit and stable:
  `phase_a`, `full_pending`, `full`, `failed`.
- Keep returning the first snapshot from phase-A instead of waiting for full
  commit-refiner construction.
- When full warmup completes during active composition, attach the commit
  refiner but defer full live-session replacement until composition is empty.
- Keep long phrase refinement debounced and stale-checked.
- Ignore key-release callbacks before canceling or rescheduling debounce work.
- Do not clear preedit on Enter until a non-empty commit response is available.
- Do not run full Hybrid decoding on every keypress.
- Measure bridge process start, session bootstrap, first key response, and first
  candidate render.

### Windows TSF

- Do not block TSF edit-session callbacks with heavy engine startup.
- Treat TSF activation as a platform shell event, not as permission to do all
  data loading synchronously.
- Initialize or warm the engine outside the critical key/edit callback when
  possible.
- Avoid directly coupling COM registration/loading concerns with decoder
  readiness.

### Android/iOS

- Start the keyboard service/extension quickly.
- Show raw composition or phase-A suggestions if full data is still loading.
- Respect platform memory and lifecycle constraints; background warmup may be
  cancelled or restarted.
- Keep data loading resumable and failure-tolerant.

## Recommended Initialization Lifecycle

```text
1. Process/package starts.
2. Platform adapter registers with OS and becomes minimally responsive.
3. Shared engine warmup begins.
4. Phase-A engine becomes ready:
   - lexicon/basic suggestions available;
   - raw preedit can be displayed;
   - no heavy full refinement required.
5. Full engine becomes ready:
   - khPOS/next-word/scoring resources available;
   - weighted span or Hybrid features available;
   - shadow/refinement features can run after debounce.
6. Runtime records readiness transitions and latency metrics.
7. If full warmup fails, phase-A or raw fallback remains usable.
```

Recommended states:

| State | Meaning | User-visible behavior |
| --- | --- | --- |
| `AdapterReady` | OS callbacks are registered. | Input source can be selected. |
| `Booting` | Engine warmup has started. | Raw preedit/fallback only if typing starts. |
| `PhaseAReady` | Basic lexicon suggestions are available. | Typing works with basic suggestions. |
| `FullReady` | Full data and decoder support is ready. | Full suggestions/refinement available. |
| `Failed` | Full initialization failed. | Fallback remains usable; show/log a clear error. |

## Metrics Developers Should Measure

Measure these separately per platform:

| Metric | Meaning |
| --- | --- |
| `process_start_ms` | Time from process start to adapter initialization. |
| `activation_to_adapter_ready_ms` | Time from OS activation/focus to lightweight adapter readiness. |
| `activation_to_phase_a_ready_ms` | Time until basic suggestions can work. |
| `activation_to_full_ready_ms` | Time until full decoder/data support is available. |
| `first_key_response_ms` | Time from first key callback to returning consumed/not-consumed. |
| `first_candidate_ms` | Time from first key to visible candidate update. |
| `hot_key_p95_ms` | P95 key processing latency during normal typing. |
| `refinement_latency_ms` | Debounced refinement request to applied response. |
| `data_parse_ms` | Time spent parsing/validating language data. |
| `index_build_ms` | Time spent building search/ranking/composer structures. |

For Linux IBus specifically, log:

- Python engine initialization start/end;
- bridge process spawn time;
- Rust phase-A session start/end;
- full warmup start/end/failure;
- full upgrade applied/deferred;
- Enter wait-for-refiner start/timeout;
- first snapshot response time;
- first `process_key_event` round-trip time;
- refinement applied/stale/retry decisions.

## Implementation Checklist For Future Fixes

- Add readiness state before adding more platform-specific performance patches.
- Identify which data is required for phase-A versus full readiness.
- Make first key behavior non-blocking or bounded.
- Keep full/shadow/Hybrid refinement off active keystrokes unless explicitly
  bounded by interactive config.
- Add stale-request checks for every async/debounced decode result.
- Make platform key-release callbacks non-mutating unless the platform requires
  a specific release action.
- Never clear active preedit before a commit response is confirmed.
- Add timeout and fallback behavior at platform bridge boundaries.
- Prefer compiled binary data over runtime CSV parsing for production paths.
- Keep platform adapters from duplicating decoder logic.
- Add tests for stale refinement, explicit selection preservation, and fallback
  behavior when full engine is not ready.
- Add manual smoke checks that include first activation, not only steady-state
  typing.

## Open Questions

- Should `ImeSession` expose readiness/fallback snapshots directly, or should
  readiness live only in platform bridge layers?
- Can the live IBus segmented preview avoid `shadow_observation` on every key
  for short inputs without losing important UX?
- Should the Rust bridge keep one global warmed engine per process or per IBus
  engine instance?
- What latency budget should KhmerIME enforce for first key response on IBus and
  Windows TSF?
- Should mobile platforms support external data warmup or only bundled data in
  the first implementation?
- Where should cross-platform performance telemetry live so Web, Linux, Windows,
  and mobile can share metric names?
