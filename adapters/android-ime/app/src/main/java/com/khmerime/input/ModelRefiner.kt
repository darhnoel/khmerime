package com.khmerime.input

// ModelRefiner
// ============
// Debounced, off-hot-path session op. Two users share it:
//   • Smart-mode model refine — re-decode the composition with the model after a pause.
//   • Candidate recompute — run the deferred decode after a typing pause
//     (see KhmerInputHandler's deferred path).
//
// Two guards keep a slow async op from ever clobbering newer input:
//   1. Debounce  — a new schedule() cancels the pending one, so only the last
//                  keystroke's op survives the pause.
//   2. Revision  — the op captures the revision at schedule time; if another
//                  schedule() bumped it before the session result comes back, the
//                  result is dropped. (The Rust side also drops a stale expectedRaw.)

const val MODEL_REFINE_DEBOUNCE_MS = 300L
const val RECOMPUTE_DEBOUNCE_MS = 200L

class ModelRefiner(
    private val session: KhmerImeSession,
    private val dispatcher: KhmerDispatcher,
    private val debounceMs: Long = MODEL_REFINE_DEBOUNCE_MS,
    // The session call to run on pause. Defaults to the model refine; the deferred
    // path passes recomputeNow. `expectedRaw` is the roman captured at schedule time.
    private val op: (KhmerImeSession, String) -> KhmerRenderState = { s, raw -> s.refineWithModel(raw) },
    private val onRefined: (KhmerRenderState) -> Unit,
) {
    private var revision = 0
    private var pending: Cancellable? = null

    // Schedule the op for the current composition. `expectedRaw` is the roman
    // captured now; the debounce and revision guards drop it if newer input arrives.
    fun schedule(expectedRaw: String) {
        pending?.cancel()
        revision++
        val scheduledRevision = revision
        pending = dispatcher.afterDelay(debounceMs) {
            dispatcher.onSession {
                val state = op(session, expectedRaw)
                dispatcher.onMain {
                    if (scheduledRevision == revision) onRefined(state)
                }
            }
        }
    }

    // Drop any pending op (e.g. on commit or focus-out).
    fun cancel() {
        pending?.cancel()
        pending = null
        revision++
    }
}
