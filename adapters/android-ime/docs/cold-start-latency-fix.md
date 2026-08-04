# Android cold-start latency fix (AI build)

## Symptom
The keyboard froze the UI 5–10 s **on every open**, worst on the first cold
start (Pixel A7). iOS, running the same neural model, did not.

## Root cause — THREE compounding costs, all on the main thread

Measured with timing probes (`nativeCreate`, `onCreateInputView`) + `Davey`
frames from logcat:

1. **Model arming** (`AiModelInitializer.onCreate`, a `ContentProvider`): extracts
   ~23 MB of model assets to `filesDir` on first launch (candle needs real file
   paths — it cannot mmap from inside the APK). Ran on the main thread at launch.
2. **Per-open lexicon rebuild**: `KhmerImeInputMethodService.session` was a
   **per-instance** field. The IME framework recreates the service on each
   show/hide, so `KhmerImeSession()` rebuilt the full lexicon + stats (~1.5 s)
   **every open**. Worse: `onStartInput` → `set_model_mode(true)` on a fresh
   session rebuilt **again with the smart refiner** (~3 s) — the Rust guard
   "skip if already in this mode" never fired because the session was new each
   time. Two `from_default_data` parses per open = the ~3 s Davey.
3. **First build blocked the main thread**: even once made a singleton, the first
   ~1.5 s build ran on the UI thread.

`onCreateInputView` itself was only ~10 ms — our view code was never the problem.

## Why iOS was fine (same model)
iOS app-bundle files are already loose on disk (no extraction), its arming is
`lazy`, and its session is process-shared. Android had none of these.

## Fixes (all measured to eliminate the freeze)

1. **Background the model arming** — `AiModelInitializer.onCreate` starts `arm()`
   on a daemon thread; `arm()` is `@Synchronized` + `@Volatile armed`. Launch no
   longer blocks on the 23 MB extraction.
2. **Process-wide session singleton** — `sharedSession` in the service companion,
   built once per process and reused across every service instance. This makes
   `set_model_mode`'s guard actually work → the lexicon builds **once**, not per
   open. Killed both the 1.5 s and the 3 s per-open rebuild.
3. **Background the first session build** — `ensureSession(onReady, mainHandler)`
   builds the singleton on a `khmer-session-build` daemon thread; `onStartInput`
   wires the handler when it lands. The keyboard renders instantly (12 ms) and
   typing attaches a moment later.

## Why it's safe (no typing regression)
- The keystroke hot path is Standard (lexicon + fuzzy) regardless of model mode;
  the model never touches typing.
- All key paths call `handler?.…` (safe-call). Before the background session
  build finishes, `handler == null` → keystrokes are no-ops, not crashes. The
  window is the first ~1.5 s of the first-ever open only.

## Result (measured)
| | before | after |
|---|---|---|
| every keyboard open | 5–6 s freeze | instant (12 ms view render) |
| first cold open | 10 s | instant; session builds in background |
| lexicon rebuilds | every open | once per process |
| main-thread Davey | 3–4.5 s | none |

## Follow-ups (not done)
- **Zero-copy lexicon** (`from_dictionary_image` already exists for iOS) would cut
  the ~1.5 s background build itself, not just move it off-thread.
- **Shrink the model** (fp32 → int8, 18.6 MB → ~5 MB) eases first-install
  extraction and the iOS 77 MB extension cap.
