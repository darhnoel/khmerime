package com.khmerime

import org.junit.Assert.*
import org.junit.Test

class GlassColorSpecTest {

    private fun Int.alpha() = (this ushr 24) and 0xFF
    private fun Int.luminance() = ((this ushr 16) and 0xFF) + ((this ushr 8) and 0xFF) + (this and 0xFF)

    @Test
    fun backgroundColorIsTranslucentInLightMode() {
        val color = GlassColorSpec.backgroundColor(isDark = false)
        assertTrue(
            "light background must be translucent (alpha < 255), got alpha=${color.alpha()}",
            color.alpha() in 1..254,
        )
    }

    @Test
    fun backgroundColorIsTranslucentInDarkMode() {
        val color = GlassColorSpec.backgroundColor(isDark = true)
        assertTrue(
            "dark background must be translucent (alpha < 255), got alpha=${color.alpha()}",
            color.alpha() in 1..254,
        )
    }

    @Test
    fun darkModeBackgroundIsDarkerThanLightMode() {
        val light = GlassColorSpec.backgroundColor(isDark = false)
        val dark = GlassColorSpec.backgroundColor(isDark = true)
        assertTrue(
            "dark background RGB sum must be less than light; light=${light.luminance()} dark=${dark.luminance()}",
            dark.luminance() < light.luminance(),
        )
    }

    @Test
    fun borderColorIsTranslucent() {
        val color = GlassColorSpec.borderColor(isDark = false)
        assertTrue(
            "border must be translucent (alpha < 255), got alpha=${color.alpha()}",
            color.alpha() in 1..254,
        )
    }

    @Test
    fun blurRadiusScalesWithDensity() {
        val radius1x = GlassColorSpec.blurRadiusPx(density = 1f)
        val radius2x = GlassColorSpec.blurRadiusPx(density = 2f)
        assertEquals(
            "blur radius must scale linearly with density",
            radius2x, radius1x * 2f, 0.01f,
        )
    }

    @Test
    fun blurRadiusIsPositive() {
        assertTrue(
            "blur radius must be > 0",
            GlassColorSpec.blurRadiusPx(density = 1f) > 0f,
        )
    }
}
