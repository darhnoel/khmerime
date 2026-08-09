package com.khmerime.views

// Screen-space bounds captured when a key first owns a gesture. Unlike local
// MotionEvent coordinates, these remain valid if suggestion rows relayout the
// keyboard before ACTION_UP arrives.
data class PressedKeyBounds(
    val left: Float,
    val top: Float,
    val right: Float,
    val bottom: Float,
) {
    fun contains(rawX: Float, rawY: Float, slop: Float): Boolean =
        rawX in left - slop..right + slop && rawY in top - slop..bottom + slop

    companion object {
        fun fromDown(
            rawX: Float,
            rawY: Float,
            localX: Float,
            localY: Float,
            width: Int,
            height: Int,
        ): PressedKeyBounds {
            val left = rawX - localX
            val top = rawY - localY
            return PressedKeyBounds(left, top, left + width, top + height)
        }
    }
}
