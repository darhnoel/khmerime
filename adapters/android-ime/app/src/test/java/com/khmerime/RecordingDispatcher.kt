package com.khmerime

import com.khmerime.input.Cancellable
import com.khmerime.input.KhmerDispatcher

// Test double that runs onSession/onMain work inline (so assertions can run
// synchronously afterward) while recording how many times each method was
// invoked. Delayed work is captured, not run — the test fires it via
// runPendingDelayed() so debounce/cancellation are deterministic.
class RecordingDispatcher : KhmerDispatcher {
    var onSessionCalls = 0
    var onMainCalls = 0
    var afterDelayCalls = 0
    private var pendingDelayed: (() -> Unit)? = null

    override fun onSession(work: () -> Unit) {
        onSessionCalls++
        work()
    }

    override fun onMain(work: () -> Unit) {
        onMainCalls++
        work()
    }

    override fun afterDelay(millis: Long, work: () -> Unit): Cancellable {
        afterDelayCalls++
        pendingDelayed = work
        return Cancellable { if (pendingDelayed === work) pendingDelayed = null }
    }

    // Fires the currently-pending delayed task, if any (simulates the timer elapsing).
    fun runPendingDelayed() {
        val work = pendingDelayed ?: return
        pendingDelayed = null
        work()
    }
}
