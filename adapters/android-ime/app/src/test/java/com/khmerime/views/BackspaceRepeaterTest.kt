package com.khmerime.views

import org.junit.Assert.*
import org.junit.Test

// BackspaceRepeaterTest
// ======================
// Tests BackspaceRepeater behavior through ManualScheduler — no real timers,
// no waiting on a Handler. Each firePending() call advances time by one
// scheduled task, letting tests drive the hold/repeat cycle deterministically.
// Mirrors iOS's BackspaceRepeaterTests.

class BackspaceRepeaterTest {

    @Test
    fun endHoldBeforeInitialDelayDoesNotFire() {
        val scheduler = ManualScheduler()
        val repeater = BackspaceRepeater(scheduler = scheduler)
        var fired = false
        repeater.onFire = { fired = true }

        repeater.beginHold()
        repeater.endHold()
        scheduler.firePending()

        assertFalse(fired)
    }

    @Test
    fun beginHoldAfterInitialDelayFiresOnce() {
        val scheduler = ManualScheduler()
        val repeater = BackspaceRepeater(scheduler = scheduler)
        var count = 0
        repeater.onFire = { count++ }

        repeater.beginHold()
        scheduler.firePending()

        assertEquals(1, count)
    }

    @Test
    fun hasFiredIsFalseBeforeInitialDelay() {
        val scheduler = ManualScheduler()
        val repeater = BackspaceRepeater(scheduler = scheduler)

        repeater.beginHold()

        assertFalse(repeater.hasFired)
    }

    @Test
    fun hasFiredIsTrueAfterInitialDelay() {
        val scheduler = ManualScheduler()
        val repeater = BackspaceRepeater(scheduler = scheduler)

        repeater.beginHold()
        scheduler.firePending()

        assertTrue(repeater.hasFired)
    }

    @Test
    fun firePendingTwiceCallsOnFireTwice() {
        val scheduler = ManualScheduler()
        val repeater = BackspaceRepeater(scheduler = scheduler)
        var count = 0
        repeater.onFire = { count++ }

        repeater.beginHold()
        scheduler.firePending()
        scheduler.firePending()

        assertEquals(2, count)
    }

    @Test
    fun firePendingFiveTimesCallsOnFireFiveTimes() {
        val scheduler = ManualScheduler()
        val repeater = BackspaceRepeater(scheduler = scheduler)
        var count = 0
        repeater.onFire = { count++ }

        repeater.beginHold()
        repeat(5) { scheduler.firePending() }

        assertEquals(5, count)
    }

    @Test
    fun endHoldAfterFirstFirePreventsSecondFire() {
        val scheduler = ManualScheduler()
        val repeater = BackspaceRepeater(scheduler = scheduler)
        var count = 0
        repeater.onFire = { count++ }

        repeater.beginHold()
        scheduler.firePending()
        repeater.endHold()
        scheduler.firePending()

        assertEquals(1, count)
    }

    @Test
    fun beginHoldResetsHasFired() {
        val scheduler = ManualScheduler()
        val repeater = BackspaceRepeater(scheduler = scheduler)

        repeater.beginHold()
        scheduler.firePending()
        assertTrue(repeater.hasFired)

        repeater.beginHold()

        assertFalse("beginHold must reset hasFired", repeater.hasFired)
    }

    @Test
    fun secondBeginHoldCancelsFirstPendingTask() {
        val scheduler = ManualScheduler()
        val repeater = BackspaceRepeater(scheduler = scheduler)
        var count = 0
        repeater.onFire = { count++ }

        repeater.beginHold()
        repeater.beginHold()
        scheduler.firePending()

        assertEquals(1, count)
    }
}

// MARK: - Test Doubles

private class ManualTask(val block: () -> Unit) : BackspaceTask {
    var isCancelled = false
        private set

    override fun cancel() {
        isCancelled = true
    }
}

class ManualScheduler : BackspaceScheduler {
    private val pending = mutableListOf<ManualTask>()

    override fun schedule(delayMillis: Long, block: () -> Unit): BackspaceTask {
        val task = ManualTask(block)
        pending.add(task)
        return task
    }

    // Fires the first non-cancelled pending task. Returns true if one was found.
    fun firePending(): Boolean {
        val task = pending.firstOrNull { !it.isCancelled } ?: return false
        pending.remove(task)
        task.block()
        return true
    }
}
