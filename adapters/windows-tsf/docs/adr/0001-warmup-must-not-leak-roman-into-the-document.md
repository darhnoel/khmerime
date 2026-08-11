# Warmup must not leak roman into the document

Status: accepted

`OnKeyDown` returned `FALSE` when the background engine build had not finished, so
keys typed during a cold start were delivered to the host application as literal
roman text. The user saw committed roman instead of a **Preedit**, and because the
text was already in the document nothing could convert it afterwards. We are
adopting **Warmup Keystroke Capture** for TSF: a key that arrives before the engine
is ready is still the IME's to handle, and passthrough is a defect rather than a
fallback.

## Why this diverges from macOS

macos-imk ADR-0002 chose a single async full load with a short condvar wait, and
justified skipping Phase A on the grounds that the IMK process is long-lived, so the
warmup cost is paid once per login. That reasoning does not transfer to TSF. Windows
loads the text-service DLL into **every host application process**, so warmup is paid
again in each app the user types in — the amortisation the macOS ADR relies on does
not exist here. macos-imk ADR-0002 also asserted that its single-phase approach was
"consistent with how Windows TSF does warmup"; that was never accurate, because macOS
waits for the engine while Windows declined the key, and the claim disguised this gap
until a user reported it.

A condvar wait is also a worse trade on Windows than on macOS: the block would land
on the host application's message pump, so a slow load presents as a hung Notepad
rather than a slow IME.

## Considered options

- **Phase A, as on Linux IBus** — chosen direction. `Transliterator::from_default_phase_a_data`
  already exists, and composing on a minimal engine keeps the key consumed with no
  blocking wait.
- **Condvar wait, as on macOS** — smallest diff, rejected: blocks a host app's UI thread.
- **Consume into a raw buffer and replay once loaded** — would satisfy the contract, but
  introduces a fourth warmup strategy with its own replay semantics and no sibling
  platform to share it with.

## Measurement

Traced in Notepad on 2026-08-11 (`[warmup-trace]` lines in `C:\Temp\khmerime-tsf.log`):
total engine build 4783 ms, with 16 keys leaking into the document over a 4994 ms window.

```text
parse_lexicon                      49.3 ms
parse_khpos                        96.4 ms
parse_next_word                     0.1 ms
parse_dictionary_image              0.1 ms
ranked_lexicon.entry_frequency    155.2 ms
ranked_lexicon.entry_indexes     2824.0 ms
search_index                     1589.7 ms
build_composer                      0.3 ms
```

Two findings, in order of size.

**The dev loop was measuring an unoptimized build.** `make platform-reinstall-windows`
builds without `--release`, and the workspace defined no `[profile.dev]` overrides, so
`khmerime_core` compiled at `opt-level = 0`. `entry_indexes` and `search_index` are
arithmetic-heavy index construction and pay for that heavily. An A/B on the same
binary — `khmerime_core` at `opt-level = 0` versus `3`, everything else held constant —
measured 5150 ms versus 1472 ms end to end. The workspace now sets `opt-level = 3` for
`khmerime_core` and all dependencies in the dev profile, leaving adapter crates
unoptimized so they stay debuggable.

**Phase A remains the right design, and the earlier concern was unfounded.** The prefix
Phase A would still pay — `parse_lexicon` plus `parse_dictionary_image` — is 49 ms, about
1% of the build. Everything expensive (`entry_indexes`, `search_index`) is precisely what
Phase A skips. Phase A does not move this stall; it removes almost all of it.

## What was built

Phase A is constructed **synchronously on the activating thread** in
`ITfTextInputProcessor::Activate`, so `state.driver` is populated before any key event
can arrive and `OnKeyDown` never has to decline a key. Measured with the optimizer fix
in place: Phase A 15.1 ms against a full build of 1157.9 ms — 77x cheaper, small enough
that blocking the host's activation path is imperceptible. This is the same shape as the
IBus bridge, which also builds Phase A synchronously at startup.

The full engine builds on a background thread and is swapped in via
`ImeSession::replace_engines`, gated on `composition_is_empty()`. A swap arriving
mid-composition is held in `pending_engine` and applied on the next command that leaves
the composition idle — swapping under an active **Composition** would re-decode the
user's in-flight input against a different engine. `DriverReadiness` tracks
`PhaseA → FullPending → Full`, mirroring the bridge's states.

The `OnKeyDown` passthrough branch survives, but it is now a failure path rather than
the warmup path: reaching it means the Phase A build itself failed. It keeps logging
`[warmup-trace] passthrough`, which should now never appear in a healthy trace.

## Consequences

The optimizer fix alone reduced cold start by roughly 3.5x but did not satisfy
**Warmup Keystroke Capture** — about a second of leak window remained. Phase A closes
it: the window is now the 15 ms Phase A build, during which no key events are delivered
yet.

Candidate quality is briefly degraded rather than absent. Phase A runs the legacy
decoder without the ranked lexicon or the full **Search Index**, so the first second of
typing after activation may rank differently than it will once the swap lands. That is a
deliberate trade against leaking roman into the document, and it matches Linux.

Any future cold-start measurement on Windows must confirm which DLL is registered and
whether it was optimized. The registered COM server path is under
`HKLM:\SOFTWARE\Classes\CLSID\{79F0A9C7-FEC5-4637-9D9D-4DFC54C8B5C2}\InprocServer32`,
and the dev loop registers a copy from `target/windows-tsf-deploy/<stamp>/`, not the MSI
build. A debug DLL is roughly 44 MB against roughly 17 MB for release.
