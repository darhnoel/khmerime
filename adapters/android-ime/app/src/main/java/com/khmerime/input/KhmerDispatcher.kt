package com.khmerime.input

import android.os.Handler
import android.os.Looper
import java.util.concurrent.Executors

// KhmerDispatcher
// ===============
// Separates "where session work runs" from the input logic in
// KhmerInputHandler. Mirrors iOS's KeyboardDispatcher.
//
//   QueuedDispatcher      — production: session work on a single background
//                           thread, main callbacks on the main Looper. Frees
//                           the UI thread from JNI/segmentation latency per key.
//
//   SynchronousDispatcher — tests: both blocks run inline on the calling
//                           thread, so the existing JVM-only test suite stays
//                           synchronous without needing to await a background thread.

interface KhmerDispatcher {
    // Runs `work` on the session processing thread (serial, background in production).
    fun onSession(work: () -> Unit)

    // Runs `work` on the main thread (or inline in tests).
    // Always called from within an onSession block.
    fun onMain(work: () -> Unit)
}

// MARK: - Production

class QueuedDispatcher : KhmerDispatcher {
    private val executor = Executors.newSingleThreadExecutor()
    private val mainHandler = Handler(Looper.getMainLooper())

    override fun onSession(work: () -> Unit) {
        executor.execute { work() }
    }

    override fun onMain(work: () -> Unit) {
        mainHandler.post { work() }
    }
}

// MARK: - Tests

class SynchronousDispatcher : KhmerDispatcher {
    override fun onSession(work: () -> Unit) = work()
    override fun onMain(work: () -> Unit) = work()
}
