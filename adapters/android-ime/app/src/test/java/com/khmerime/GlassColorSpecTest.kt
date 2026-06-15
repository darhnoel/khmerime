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

    @Test
    fun selectedCandidateBackgroundHasHigherAlphaThanRegularInDarkMode() {
        val regular = GlassColorSpec.backgroundColor(isDark = true)
        val selected = GlassColorSpec.selectedCandidateBackground(isDark = true)
        assertTrue(
            "selected pill must be more opaque than regular; regular=${regular.alpha()} selected=${selected.alpha()}",
            selected.alpha() > regular.alpha(),
        )
    }

    @Test
    fun selectedCandidateBackgroundHasHigherAlphaThanRegularInLightMode() {
        val regular = GlassColorSpec.backgroundColor(isDark = false)
        val selected = GlassColorSpec.selectedCandidateBackground(isDark = false)
        assertTrue(
            "selected pill must be more opaque than regular; regular=${regular.alpha()} selected=${selected.alpha()}",
            selected.alpha() > regular.alpha(),
        )
    }

    @Test
    fun selectedCandidateBackgroundIsNotFullyOpaque() {
        val selected = GlassColorSpec.selectedCandidateBackground(isDark = true)
        assertTrue(
            "selected pill must stay translucent (glass effect), got alpha=${selected.alpha()}",
            selected.alpha() < 255,
        )
    }

    @Test
    fun candidateBorderWidthIsWiderThanKeyBorderWidth() {
        assertTrue(
            "candidate border must be visually obvious (> 1dp equivalent)",
            GlassColorSpec.candidateBorderWidth() > 1f,
        )
    }
}
