package com.khmerime.views

import android.os.SystemClock
import android.view.MotionEvent
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.khmerime.layout.KeyboardKey
import com.khmerime.layout.KeyboardKeyAction
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class GlassKeyViewTouchTest {
    @Test
    fun touchDownPerformsOneKeyboardHaptic() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()

        instrumentation.runOnMainSync {
            var haptics = 0
            val view = object : GlassKeyView(
                context = instrumentation.targetContext,
                key = KeyboardKey("A", KeyboardKeyAction.Insert, "a"),
                onClick = {},
            ) {
                override fun performKeyPressHaptic() {
                    haptics += 1
                }
            }.apply {
                layout(0, 0, 100, 100)
            }
            val downTime = SystemClock.uptimeMillis()
            val down = MotionEvent.obtain(
                downTime,
                downTime,
                MotionEvent.ACTION_DOWN,
                50f,
                50f,
                0,
            )
            val up = MotionEvent.obtain(
                downTime,
                downTime + 20,
                MotionEvent.ACTION_UP,
                50f,
                50f,
                0,
            )

            try {
                view.onTouchEvent(down)
                view.onTouchEvent(up)
                assertEquals(1, haptics)
            } finally {
                down.recycle()
                up.recycle()
            }
        }
    }

    @Test
    fun releaseUsesTouchSlopWithoutCommittingFarDrift() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        var commits = 0

        instrumentation.runOnMainSync {
            val view = GlassKeyView(
                context = instrumentation.targetContext,
                key = KeyboardKey("A", KeyboardKeyAction.Insert, "a"),
                onClick = { commits += 1 },
            ).apply {
                layout(0, 0, 100, 100)
            }
            val downTime = SystemClock.uptimeMillis()
            val down = MotionEvent.obtain(
                downTime,
                downTime,
                MotionEvent.ACTION_DOWN,
                50f,
                50f,
                0,
            )
            val upOutside = MotionEvent.obtain(
                downTime,
                downTime + 20,
                MotionEvent.ACTION_UP,
                104f,
                50f,
                0,
            )
            val secondDown = MotionEvent.obtain(
                downTime + 40,
                downTime + 40,
                MotionEvent.ACTION_DOWN,
                50f,
                50f,
                0,
            )
            val upFarOutside = MotionEvent.obtain(
                downTime + 40,
                downTime + 60,
                MotionEvent.ACTION_UP,
                150f,
                50f,
                0,
            )

            try {
                view.onTouchEvent(down)
                assertEquals("touch-down must remain correctable", 0, commits)

                view.onTouchEvent(upOutside)
                assertEquals("small lift drift must commit exactly once", 1, commits)

                view.onTouchEvent(secondDown)
                view.onTouchEvent(upFarOutside)
                assertEquals("a deliberate slide away must cancel the wrong key", 1, commits)
            } finally {
                down.recycle()
                upOutside.recycle()
                secondDown.recycle()
                upFarOutside.recycle()
            }
        }
    }
}
