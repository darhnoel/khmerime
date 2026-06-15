package com.khmerime

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class KeyboardPresentationSpecTest {
    @Test
    fun suggestCharacterCandidatesStayInSuggestionBarWithoutReplacingQwertyLayout() {
        val state = KhmerRenderState(
            candidates = listOf("ក", "ខ"),
            preedit = "k",
        )

        assertEquals(
            "Suggest Character suggestions must show mapped Khmer candidates",
            listOf("ក", "ខ"),
            KeyboardPresentationSpec.suggestionCandidates(state),
        )
        assertEquals(
            "Suggest Character is direct character picking, not roman composition",
            "",
            KeyboardPresentationSpec.preeditText(KeyboardState.SuggestCharacter, state),
        )
        assertFalse(
            "Suggest Character render state must not replace the QWERTY keyboard layer",
            KeyboardPresentationSpec.renderStateReplacesKeyboardLayer(KeyboardState.SuggestCharacter),
        )
        assertEquals(
            "Suggest Character mode keeps the default QWERTY keyboard layout",
            KeyboardLayer.Qwerty,
            KeyboardPresentationSpec.keyboardLayerForState(KeyboardState.SuggestCharacter),
        )
        assertFalse(
            "Android does not replace the keyboard layer with candidate panels",
            KeyboardPresentationSpec.renderStateReplacesKeyboardLayer(KeyboardState.Panel),
        )
    }
}
