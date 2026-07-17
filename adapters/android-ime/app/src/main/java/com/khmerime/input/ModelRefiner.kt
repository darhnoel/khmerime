package com.khmerime.input

// ModelRefiner
// ============
// Debounced, off-hot-path model refine. Mirrors the iOS Visible Refiner:
// keystrokes never run the model; instead each keystroke (in Smart mode)
// reschedules a refine, and only after the typing pauses does the model
// re-decode the composition.
//
// Two guards keep a slow async refine from ever clobbering newer input:
//   1. Debounce  — a new schedule() cancels the pending one, so only the last
//                  keystroke's refine survives the pause.
//   2. Revision  — the refine captures the revision at schedule time; if another
//                  schedule() bumped it before the session result comes back, the
//                  result is dropped. (The Rust side also drops a stale expectedRaw.)

const val MODEL_REFINE_DEBOUNCE_MS = 300L

class ModelRefiner(
    private val session: KhmerImeSession,
    private val dispatcher: KhmerDispatcher,
    private val onRefined: (KhmerRenderState) -> Unit,
) {
    private var revision = 0
    private var pending: Cancellable? = null

    // Schedule a refine for the current composition. `expectedRaw` is the roman
    // captured now; the debounce and revision guards drop it if newer input arrives.
    fun schedule(expectedRaw: String) {
        pending?.cancel()
        revision++
        val scheduledRevision = revision
        pending = dispatcher.afterDelay(MODEL_REFINE_DEBOUNCE_MS) {
            dispatcher.onSession {
                val state = session.refineWithModel(expectedRaw)
                dispatcher.onMain {
                    if (scheduledRevision == revision) onRefined(state)
                }
            }
        }
    }

    // Drop any pending refine (e.g. on commit or focus-out).
    fun cancel() {
        pending?.cancel()
        pending = null
        revision++
    }
}
