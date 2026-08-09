package com.khmerime.views

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PressedKeyBoundsTest {
    @Test
    fun unchangedScreenPositionSurvivesKeyboardRelayout() {
        val bounds = PressedKeyBounds.fromDown(
            rawX = 920f,
            rawY = 1767f,
            localX = 59f,
            localY = 81f,
            width = 107,
            height = 141,
        )

        assertTrue(
            "the captured O stayed at the same screen coordinate while its local y became -77",
            bounds.contains(rawX = 920f, rawY = 1767f, slop = 21f),
        )
    }

    @Test
    fun deliberateSlideStillCancelsThePressedKey() {
        val bounds = PressedKeyBounds.fromDown(
            rawX = 50f,
            rawY = 50f,
            localX = 50f,
            localY = 50f,
            width = 100,
            height = 100,
        )

        assertFalse(bounds.contains(rawX = 150f, rawY = 50f, slop = 8f))
    }
}
