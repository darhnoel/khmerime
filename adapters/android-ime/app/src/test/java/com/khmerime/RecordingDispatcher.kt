package com.khmerime

import com.khmerime.input.KhmerDispatcher

// Test double that runs work inline (so assertions can run synchronously
// afterward) while recording how many times each method was invoked.
class RecordingDispatcher : KhmerDispatcher {
    var onSessionCalls = 0
    var onMainCalls = 0

    override fun onSession(work: () -> Unit) {
        onSessionCalls++
        work()
    }

    override fun onMain(work: () -> Unit) {
        onMainCalls++
        work()
    }
}
