package com.khmerime

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
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

    @Test
    fun toggleIsActiveWhenKeyIsPanelAndStateIsSuggestCharacter() {
        val key = KeyboardKey("✦", KeyboardKeyAction.TogglePanel)
        assertTrue(
            "✦ key must be active when state is SuggestCharacter",
            KeyboardPresentationSpec.isToggleActive(key, KeyboardState.SuggestCharacter),
        )
    }

    @Test
    fun toggleIsInactiveWhenStateIsQwerty() {
        val key = KeyboardKey("✦", KeyboardKeyAction.TogglePanel)
        assertFalse(
            "✦ key must be inactive when state is Qwerty",
            KeyboardPresentationSpec.isToggleActive(key, KeyboardState.Qwerty),
        )
    }

    @Test
    fun nonToggleKeyIsNeverActive() {
        val key = KeyboardKey("A", KeyboardKeyAction.Insert, "a")
        assertFalse(
            "non-TogglePanel key must never report active",
            KeyboardPresentationSpec.isToggleActive(key, KeyboardState.SuggestCharacter),
        )
    }

    @Test
    fun preeditIsEmptyInEnglishMode() {
        val state = KhmerRenderState(candidates = emptyList(), preedit = "hi")
        assertEquals(
            "preedit must be empty in English mode",
            "",
            KeyboardPresentationSpec.preeditText(KeyboardState.English, state),
        )
    }

    @Test
    fun englishToggleIsActiveWhenStateIsEnglish() {
        val key = KeyboardKey("En", KeyboardKeyAction.ToggleEnglish)
        assertTrue(
            "En key must be active when state is English",
            KeyboardPresentationSpec.isKeyActive(key, KeyboardState.English),
        )
    }

    @Test
    fun englishToggleIsInactiveWhenStateIsQwerty() {
        val key = KeyboardKey("En", KeyboardKeyAction.ToggleEnglish)
        assertFalse(
            "En key must be inactive when state is Qwerty",
            KeyboardPresentationSpec.isKeyActive(key, KeyboardState.Qwerty),
        )
    }

    @Test
    fun panelToggleIsActiveViaIsKeyActiveWhenStateIsSuggestCharacter() {
        val key = KeyboardKey("✦", KeyboardKeyAction.TogglePanel)
        assertTrue(
            "✦ key must be active via isKeyActive when state is SuggestCharacter",
            KeyboardPresentationSpec.isKeyActive(key, KeyboardState.SuggestCharacter),
        )
    }

    @Test
    fun insertKeyIsNeverActiveViaIsKeyActive() {
        val key = KeyboardKey("A", KeyboardKeyAction.Insert, "a")
        assertFalse(
            "Insert key must never be active",
            KeyboardPresentationSpec.isKeyActive(key, KeyboardState.English),
        )
    }

    @Test
    fun selectedCandidateIndexReturnsNullWhenNoneSelected() {
        val state = KhmerRenderState(candidates = listOf("ក", "ខ"), selectedIndex = null)
        assertNull(
            "selectedCandidateIndex must be null when session has no selection",
            KeyboardPresentationSpec.selectedCandidateIndex(state),
        )
    }

    @Test
    fun selectedCandidateIndexReturnsIntFromState() {
        val state = KhmerRenderState(candidates = listOf("ក", "ខ"), selectedIndex = 1)
        assertEquals(
            "selectedCandidateIndex must reflect session selectedIndex",
            1 as Int?,
            KeyboardPresentationSpec.selectedCandidateIndex(state),
        )
    }

    @Test
    fun selectedCandidateIndexZeroIsDistinctFromNull() {
        val state = KhmerRenderState(candidates = listOf("ក", "ខ"), selectedIndex = 0)
        assertEquals(
            "selectedIndex 0 must return 0, not null",
            0 as Int?,
            KeyboardPresentationSpec.selectedCandidateIndex(state),
        )
    }
}
