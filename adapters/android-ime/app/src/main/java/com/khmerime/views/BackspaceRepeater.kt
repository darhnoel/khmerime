package com.khmerime.views

import android.os.Handler
import android.os.Looper

// BackspaceRepeater
// ==================
// The standard keyboard feel where holding ⌫ fires one delete per 100 ms
// after a 400 ms initial delay. Isolated from any Android View so it can be
// tested without a Handler/Looper. Mirrors iOS's BackspaceRepeater.

interface BackspaceTask {
    fun cancel()
}

interface BackspaceScheduler {
    fun schedule(delayMillis: Long, block: () -> Unit): BackspaceTask
}

// MARK: - Production scheduler (main Looper)

class HandlerBackspaceScheduler : BackspaceScheduler {
    private val handler = Handler(Looper.getMainLooper())

    override fun schedule(delayMillis: Long, block: () -> Unit): BackspaceTask {
        val runnable = Runnable(block)
        handler.postDelayed(runnable, delayMillis)
        return object : BackspaceTask {
            override fun cancel() = handler.removeCallbacks(runnable)
        }
    }
}

// MARK: - BackspaceRepeater

class BackspaceRepeater(
    private val initialDelayMs: Long = 400,
    private val repeatIntervalMs: Long = 100,
    private val scheduler: BackspaceScheduler = HandlerBackspaceScheduler(),
) {
    var onFire: (() -> Unit)? = null

    var hasFired: Boolean = false
        private set

    private var pendingTask: BackspaceTask? = null

    fun beginHold() {
        hasFired = false
        pendingTask?.cancel()
        pendingTask = scheduler.schedule(initialDelayMs) { fire() }
    }

    fun endHold() {
        pendingTask?.cancel()
        pendingTask = null
    }

    private fun fire() {
        hasFired = true
        onFire?.invoke()
        pendingTask = scheduler.schedule(repeatIntervalMs) { fire() }
    }
}
