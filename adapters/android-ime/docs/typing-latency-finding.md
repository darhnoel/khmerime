# Typing latency — diagnosis (fix pending)

## Symptom
After the cold-start load fix (keyboard now opens instantly), **typing itself
lags** — 300–700 ms per keystroke, growing with composition length. Typing a
long word fast (`geaveakreakkech`) shows the candidate strip churning through
intermediate results, one slow update per key.

## Measured cause — the base decoder, NOT the AI
Per-keystroke timing probe on `processCharacter` (roman → Khmer JNI call):

```
short buffer  ~25 ms
'geav…'       ~300 ms
'geaveakr…'   ~500–740 ms   (grows with length)
```

Isolation tests (all measured on-device):
- **Standard mode (Smart OFF, model absent): identical lag.** So it is **not**
  the neural model. `nativeRefineWithModel` never fired during typing either.
- **Not the cold-start fixes** — `processCharacter`/`snapshot` are untouched by
  those (session singleton + background build).
- **Not the `wfst_max_latency_ms` budget** — lowering it 250 ms → 75 ms did
  **not** help. The cost is the decode *work*, not its cap.

Localisation: `snapshot()` only clones already-computed candidates (cheap). The
expensive **weighted-span decode runs synchronously inside
`ImeSession::process_key_event`** — on every keystroke, over the whole growing
composition. This is the "multi-second suggest() on long input" the public
Makefile warns about, just at interactive length.

## The fix (user-identified, correct): debounce the candidate decode
The user's own observation is the right design: when typing fast, only the
**final** composition's candidates matter; the intermediate per-key decodes are
wasted work that causes the lag.

Split the keystroke path:
1. **Insert the roman char + apply any commit immediately** (must stay instant).
2. **Defer the expensive candidate decode** — run it ~100–150 ms after typing
   pauses, cancelling superseded decodes (exactly what `ModelRefiner` already
   does for the model, `MODEL_REFINE_DEBOUNCE_MS = 300`).

Fast typing then pays the decode **once** (on pause), not per key.

## Why it wasn't done in this pass
`process_key_event` currently couples commit-critical work with the deferrable
candidate decode. Doing this safely needs an **engine API change**: a way to
process a key (insert + commit) *without* the full candidate decode, plus a
separate "decode candidates now" call to run debounced. That is a careful Rust
change to `crates/session` — not a config tweak — so it is scoped here rather
than rushed.

## Next step
Add to `ImeSession` a light key-process path (mutate composition + return commit,
skip candidate decode) and a `decode_candidates()` call. Then in
`KhmerInputHandler.sendChar`, insert immediately and debounce `decode_candidates`
like the model refiner. Keep commit synchronous so auto-commit still feels
instant.

## Update: debounce was tried and REJECTED (2026-08-04)

Debouncing the decode (append char via a no-decode path, run the decode ~90 ms
after a typing pause) was implemented and measured on-device — **it made normal
typing worse** and was reverted.

Why it failed: each decode (`recompute_composition_state`) is **300–800 ms**
regardless of when it runs. Normal typing has >90 ms between keys, so every key
still triggered its own decode — now with an added 90 ms delay before it. The
debounce only helps typists faster than the window, which real users are not.

**The real enemy is the per-decode cost itself (300–800 ms), not its timing.**
Debouncing cannot fix that. Before any further attempt, PROFILE which phase of
`recompute_composition_state` eats the time (segmentation vs candidate build vs
WFST search — note `wfst_max_latency_ms=250` did NOT change it, so the cost is
likely upstream of the WFST cap). Fix at the source (make the decode faster or
incremental), not by moving when it runs.
