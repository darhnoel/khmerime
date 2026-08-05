package com.khmerime.views

import android.view.View
import android.widget.FrameLayout
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class KeyPreviewOverlayTest {
    @Test
    fun nestedKeyProducesStableKeyboardLocalPreviewFrame() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()

        instrumentation.runOnMainSync {
            val context = instrumentation.targetContext
            val root = FrameLayout(context)
            val keyboard = FrameLayout(context)
            val key = View(context)
            val overlay = KeyPreviewOverlay(context).apply {
                visibility = View.GONE
            }
            keyboard.addView(key)
            root.addView(keyboard)
            root.addView(overlay)

            root.layout(0, 0, 360, 340)
            keyboard.layout(0, 100, 360, 340)
            key.layout(100, 20, 140, 70)

            overlay.show(key, "A")

            val frame = requireNotNull(overlay.previewFrame)
            assertEquals(360, overlay.width)
            assertEquals(340, overlay.height)
            assertEquals(340, root.height)
            assertEquals(89f, frame.left, 0.001f)
            assertEquals(46.5f, frame.top, 0.001f)
            assertTrue("preview must stay above the key", frame.bottom <= 120f)
        }
    }

    @Test
    fun unmeasuredOverlayDoesNotCrash() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()

        instrumentation.runOnMainSync {
            val context = instrumentation.targetContext
            val root = FrameLayout(context)
            val key = View(context)
            val overlay = KeyPreviewOverlay(context)
            root.addView(key)
            root.addView(overlay)

            key.layout(100, 20, 140, 70)

            overlay.show(key, "A")

            assertNull(overlay.previewFrame)
        }
    }
}
