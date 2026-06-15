package com.khmerime

object GlassColorSpec {
    fun backgroundColor(isDark: Boolean): Int =
        if (isDark) argb(210, 90, 90, 100) else argb(230, 255, 255, 255)

    fun borderColor(isDark: Boolean): Int =
        if (isDark) argb(60, 255, 255, 255) else argb(40, 0, 0, 0)

    fun blurRadiusPx(density: Float): Float = 12f * density

    private fun argb(a: Int, r: Int, g: Int, b: Int): Int =
        (a and 0xFF shl 24) or (r and 0xFF shl 16) or (g and 0xFF shl 8) or (b and 0xFF)
}
