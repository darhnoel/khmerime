package com.khmerime

import com.khmerime.views.PopupRect
import com.khmerime.views.absoluteKeyPreviewPopupFrame
import com.khmerime.views.keyPreviewPopupFrame
import org.junit.Assert.assertEquals
import org.junit.Test

class KeyPreviewPopupFrameTest {
    @Test
    fun popupFloatsAboveKeySizedRelativeToIt() {
        val source = PopupRect(left = 100f, top = 200f, right = 140f, bottom = 250f)
        val bounds = PopupRect(0f, 0f, 360f, 300f)
        val frame = keyPreviewPopupFrame(source, bounds)
        val width = 40f * 1.55f
        val height = 50f * 1.35f
        assertEquals(width, frame.width, 0.001f)
        assertEquals(height, frame.height, 0.001f)
        assertEquals(120f - width / 2, frame.left, 0.001f)
        assertEquals(200f - height - 6f, frame.top, 0.001f)
    }

    @Test
    fun popupClampsToRightEdge() {
        val source = PopupRect(320f, 200f, 360f, 250f)
        val bounds = PopupRect(0f, 0f, 360f, 300f)
        val frame = keyPreviewPopupFrame(source, bounds)
        val width = 40f * 1.55f
        assertEquals(294f, frame.left, 0.001f)
        assertEquals(294f + width, frame.right, 0.001f)
    }

    @Test
    fun topRowPopupStillFloatsAboveKey() {
        val source = PopupRect(100f, 10f, 140f, 60f)
        val bounds = PopupRect(0f, 0f, 360f, 300f)
        val frame = keyPreviewPopupFrame(source, bounds)
        val height = 50f * 1.35f
        assertEquals(10f - height - 6f, frame.top, 0.001f)
        assertEquals(10f - 6f, frame.bottom, 0.001f)
    }

    @Test
    fun unmeasuredBoundsDoNotCrashGeometry() {
        val source = PopupRect(100f, 10f, 140f, 60f)
        val frame = keyPreviewPopupFrame(source, PopupRect(0f, 0f, 0f, 0f))

        assertEquals(4f, frame.left, 0.001f)
    }

    @Test
    fun topRowPopupIsTranslatedIntoScreenSpaceAboveImeWindow() {
        val frame = absoluteKeyPreviewPopupFrame(
            sourceInIme = PopupRect(100f, 4f, 140f, 54f),
            imeLeftOnScreen = 0f,
            imeTopOnScreen = 1400f,
            screenWidth = 1080f,
        )

        assertEquals(1330.5f, frame.top, 0.001f)
        assertEquals(1398f, frame.bottom, 0.001f)
    }
}
