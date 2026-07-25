package com.khmerime

import com.khmerime.views.PopupRect
import com.khmerime.views.keyPreviewPopupFrame
import org.junit.Assert.assertEquals
import org.junit.Test

// Kotlin port of iOS KeyPreviewPopupView.frame(sourceFrame:in:): a bubble floating
// above the pressed key, sized relative to it, clamped inside the keyboard bounds.
// Pure floats (no android.graphics.RectF) so it runs as a real JVM unit test.
class KeyPreviewPopupFrameTest {

    // A comfortably-sized interior key: bubble is 1.55x wide, 1.35x tall, and floats
    // verticalGap(6) above the key's top.
    @Test
    fun popupFloatsAboveKeySizedRelativeToIt() {
        val source = PopupRect(left = 100f, top = 200f, right = 140f, bottom = 250f) // 40x50
        val bounds = PopupRect(0f, 0f, 360f, 300f)
        val frame = keyPreviewPopupFrame(source, bounds)
        val width = 40f * 1.55f  // 62
        val height = 50f * 1.35f // 67.5
        assertEquals(width, frame.width, 0.001f)
        assertEquals(height, frame.height, 0.001f)
        // centered horizontally on the key: midX 120 - width/2
        assertEquals(120f - width / 2, frame.left, 0.001f)
        // sits above the key top (200) with a 6px gap
        assertEquals(200f - height - 6f, frame.top, 0.001f)
    }

    // A key hard against the right edge: the wider bubble would overflow, so it clamps
    // to edgeInset(4) from the right bound instead of centering on the key.
    @Test
    fun popupClampsToRightEdge() {
        val source = PopupRect(320f, 200f, 360f, 250f) // 40 wide, flush right of 360-wide bounds
        val bounds = PopupRect(0f, 0f, 360f, 300f)
        val frame = keyPreviewPopupFrame(source, bounds)
        val width = 40f * 1.55f // 62
        // maxX = 360 - 4 - 62 = 294; centered would be 340-31=309 (overflow) → clamped to 294
        assertEquals(294f, frame.left, 0.001f)
        assertEquals(294f + width, frame.right, 0.001f)
    }

    // A key in the top row: the bubble would float above the keyboard's top edge, so it
    // clamps down to edgeInset(4) from the top bound.
    @Test
    fun popupClampsToTopEdge() {
        val source = PopupRect(100f, 10f, 140f, 60f) // key top at 10, near the ceiling
        val bounds = PopupRect(0f, 0f, 360f, 300f)
        val frame = keyPreviewPopupFrame(source, bounds)
        // unclamped top = 10 - 67.5 - 6 = -63.5 → clamped to bounds.top + 4 = 4
        assertEquals(4f, frame.top, 0.001f)
    }
}
