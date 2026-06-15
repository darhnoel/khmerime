package com.khmerime

import org.junit.Assert.*
import org.junit.Test

class GlassKeyPressAnimatorTest {

    // Instant runner: skips ValueAnimator, fires the final value immediately.
    private val instant = AnimatorRunner { _, to, _, onUpdate -> onUpdate(to) }

    private fun makeAnimator(updates: MutableList<Float> = mutableListOf()): GlassKeyPressAnimator {
        return GlassKeyPressAnimator(onUpdate = { updates += it }, runner = instant)
    }

    @Test
    fun pressDriversSquishTowardOne() {
        val updates = mutableListOf<Float>()
        val animator = makeAnimator(updates)

        animator.press()

        assertTrue("press must fire at least one update", updates.isNotEmpty())
        assertEquals("press must animate to 1f", 1f, updates.last(), 0f)
    }

    @Test
    fun releaseDriversSquishTowardZero() {
        val updates = mutableListOf<Float>()
        val animator = makeAnimator(updates)

        animator.press()
        updates.clear()
        animator.release()

        assertTrue("release must fire at least one update", updates.isNotEmpty())
        assertEquals("release must animate to 0f", 0f, updates.last(), 0f)
    }

    @Test
    fun rapidPressThenReleaseDoesNotProduceOutOfRangeValues() {
        val updates = mutableListOf<Float>()
        val animator = makeAnimator(updates)

        repeat(10) {
            animator.press()
            animator.release()
        }

        assertTrue("all squish values must be in 0f..1f", updates.all { it in 0f..1f })
    }

    @Test
    fun multiplePressCyclesAlwaysEndAtZeroAfterRelease() {
        val updates = mutableListOf<Float>()
        val animator = makeAnimator(updates)

        repeat(3) {
            animator.press()
            animator.release()
        }

        assertEquals("squish must end at 0f after release", 0f, updates.last(), 0f)
    }
}
