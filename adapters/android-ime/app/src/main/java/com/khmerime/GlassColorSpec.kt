package com.khmerime

object GlassColorSpec {
    fun backgroundColor(isDark: Boolean): Int =
        if (isDark) argb(210, 90, 90, 100) else argb(230, 255, 255, 255)

    fun borderColor(isDark: Boolean): Int =
        if (isDark) argb(60, 255, 255, 255) else argb(40, 0, 0, 0)

    fun toggleActiveBackground(isDark: Boolean): Int =
        if (isDark) argb(245, 240, 240, 245) else argb(245, 255, 255, 255)

    fun toggleActiveTextColor(): Int = argb(255, 20, 20, 24)

    fun selectedCandidateBackground(isDark: Boolean): Int =
        if (isDark) argb(235, 110, 110, 124) else argb(245, 210, 215, 225)

    fun candidateBorderWidth(): Float = 1.5f

    fun blurRadiusPx(density: Float): Float = 12f * density

    private fun argb(a: Int, r: Int, g: Int, b: Int): Int =
        (a and 0xFF shl 24) or (r and 0xFF shl 16) or (g and 0xFF shl 8) or (b and 0xFF)
}
