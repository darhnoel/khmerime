package com.khmerime

import com.khmerime.input.KhmerRenderState
import com.khmerime.input.KeyboardState
import com.khmerime.layout.KhmerInputChrome
import com.khmerime.layout.KhmerInputChromePresentation
import org.junit.Assert.assertEquals
import org.junit.Test

class KhmerInputChromeTest {
    @Test
    fun presentationFollowsTheMobileTwoOneZeroRowContract() {
        val empty = KhmerRenderState()
        val charPickResults = KhmerRenderState(candidates = listOf("ក", "ខ"))

        val presentations = listOf(
            KhmerInputChrome.presentation(KeyboardState.Qwerty, "", empty),
            KhmerInputChrome.presentation(KeyboardState.Qwerty, "nhom", empty),
            KhmerInputChrome.presentation(KeyboardState.SuggestCharacter, "", empty),
            KhmerInputChrome.presentation(KeyboardState.SuggestCharacter, "", charPickResults),
            KhmerInputChrome.presentation(KeyboardState.English, "", empty),
        )

        assertEquals(
            listOf(
                KhmerInputChromePresentation.QuickAccess,
                KhmerInputChromePresentation.Composition,
                KhmerInputChromePresentation.CharPickQuickAccess,
                KhmerInputChromePresentation.CharPickCandidates,
                KhmerInputChromePresentation.Hidden,
            ),
            presentations,
        )
        assertEquals(listOf(2, 2, 1, 1, 0), presentations.map { it.rowCount })
    }
}
