package com.khmerime.views

import android.animation.ValueAnimator
import android.view.animation.DecelerateInterpolator

fun interface AnimatorRunner {
    fun run(from: Float, to: Float, durationMs: Long, onUpdate: (Float) -> Unit)
}

class GlassKeyPressAnimator(
    private val onUpdate: (squish: Float) -> Unit,
    private val runner: AnimatorRunner = defaultRunner(),
) {
    private var squish = 0f

    fun press() = runner.run(squish, 1f, PRESS_MS) { squish = it; onUpdate(it) }
    fun release() = runner.run(squish, 0f, RELEASE_MS) { squish = it; onUpdate(it) }

    companion object {
        private const val PRESS_MS = 80L
        private const val RELEASE_MS = 220L

        fun defaultRunner(): AnimatorRunner = AnimatorRunner { from, to, duration, onUpdate ->
            ValueAnimator.ofFloat(from, to).apply {
                this.duration = duration
                interpolator = DecelerateInterpolator()
                addUpdateListener { onUpdate(it.animatedValue as Float) }
                start()
            }
        }
    }
}
