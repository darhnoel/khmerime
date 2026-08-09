package com.khmerime.views

// Pure geometry for the keypress preview bubble. Kept free of android.graphics
// so it unit-tests on the plain JVM (no Robolectric).

data class PopupRect(val left: Float, val top: Float, val right: Float, val bottom: Float) {
    val width: Float get() = right - left
    val height: Float get() = bottom - top
    val midX: Float get() = (left + right) / 2
}

private const val EDGE_INSET = 4f
private const val VERTICAL_GAP = 6f

// The bubble floats above `source`, sized 1.55x/1.35x of the key (min 48x56),
// centered on the key, and clamped horizontally inside `bounds`. The top may be
// outside `bounds` so top-row keys still get an unobstructed preview above them.
fun keyPreviewPopupFrame(source: PopupRect, bounds: PopupRect): PopupRect {
    val width = maxOf(source.width * 1.55f, 48f)
    val height = maxOf(source.height * 1.35f, 56f)
    val minX = bounds.left + EDGE_INSET
    val maxX = bounds.right - EDGE_INSET - width
    val left = if (maxX >= minX) {
        (source.midX - width / 2).coerceIn(minX, maxX)
    } else {
        minX
    }
    val top = source.top - height - VERTICAL_GAP
    return PopupRect(left, top, left + width, top + height)
}

// PopupWindow.showAtLocation uses screen coordinates. Translate before doing the
// geometry so a top-row preview can occupy the host-app area above the IME window.
fun absoluteKeyPreviewPopupFrame(
    sourceInIme: PopupRect,
    imeLeftOnScreen: Float,
    imeTopOnScreen: Float,
    screenWidth: Float,
): PopupRect = keyPreviewPopupFrame(
    source = PopupRect(
        left = sourceInIme.left + imeLeftOnScreen,
        top = sourceInIme.top + imeTopOnScreen,
        right = sourceInIme.right + imeLeftOnScreen,
        bottom = sourceInIme.bottom + imeTopOnScreen,
    ),
    bounds = PopupRect(0f, 0f, screenWidth, Float.MAX_VALUE),
)
