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

    // ── Strip display logic ────────────────────────────────────────────────────

    private fun seg(input: String, output: String, focused: Boolean = false) =
        KhmerSegmentEntry(output = output, input = input, focused = focused)

    @Test
    fun romanRowTextShowsRomanBufferWhenNoSegments() {
        val state = KhmerRenderState(candidates = listOf("ខ្ញុំ"), preedit = "nhom")
        assertEquals(
            "roman row must show romanBuffer when no segments",
            "nhom",
            KeyboardPresentationSpec.romanRowText(state, romanBuffer = "nhom"),
        )
    }

    @Test
    fun romanRowTextShowsJoinedInputsWhenSegmentsPresent() {
        val state = KhmerRenderState(
            segments = listOf(seg("nhom", "ខ្ញុំ"), seg("ttov", "ទៅ")),
        )
        assertEquals(
            "roman row must join segment inputs with ·",
            "nhom · ttov",
            KeyboardPresentationSpec.romanRowText(state, romanBuffer = ""),
        )
    }

    @Test
    fun romanRowTextShowsEditIndicatorWhenSegmentEditActive() {
        val state = KhmerRenderState(
            segments = listOf(seg("nhom", "ខ្ញុំ"), seg("ttov", "ទៅ")),
            segmentEditActive = true,
            segmentEditIndex = 1,
        )
        assertEquals(
            "roman row must show ✏ with bracket around edited segment",
            "✏ nhom · [ttov]",
            KeyboardPresentationSpec.romanRowText(state, romanBuffer = ""),
        )
    }

    @Test
    fun segmentKhmerTextsReturnsEmptyListWhenNoSegments() {
        val state = KhmerRenderState(candidates = listOf("ខ្ញុំ"), selectedIndex = 0)
        assertTrue(
            "segmentKhmerTexts must return empty list when no segments",
            KeyboardPresentationSpec.segmentKhmerTexts(state).isEmpty(),
        )
    }

    @Test
    fun segmentKhmerTextsReturnsOutputsPerSegment() {
        val state = KhmerRenderState(
            segments = listOf(seg("nhom", "ខ្ញុំ"), seg("ttov", "ទៅ")),
        )
        assertEquals(
            "segmentKhmerTexts must return one Khmer string per segment",
            listOf("ខ្ញុំ", "ទៅ"),
            KeyboardPresentationSpec.segmentKhmerTexts(state),
        )
    }

    @Test
    fun focusedSegmentIndexReturnsNullWhenNoSegments() {
        val state = KhmerRenderState()
        assertNull(
            "focusedSegmentIndex must be null when there are no segments",
            KeyboardPresentationSpec.focusedSegmentIndex(state),
        )
    }

    @Test
    fun focusedSegmentIndexReturnsIndexFromState() {
        val state = KhmerRenderState(
            segments = listOf(seg("nhom", "ខ្ញុំ"), seg("ttov", "ទៅ")),
            focusedSegmentIndex = 1,
        )
        assertEquals(
            "focusedSegmentIndex must reflect state.focusedSegmentIndex",
            1 as Int?,
            KeyboardPresentationSpec.focusedSegmentIndex(state),
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
