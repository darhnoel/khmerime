package com.khmerime

import com.khmerime.input.KhmerPhraseCandidate
import com.khmerime.input.KhmerRenderState
import com.khmerime.input.KhmerSegmentEntry
import com.khmerime.input.KeyboardState
import com.khmerime.layout.KeyboardKey
import com.khmerime.layout.KeyboardKeyAction
import com.khmerime.layout.KeyboardLayer
import com.khmerime.layout.KeyboardPresentationSpec
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
            KeyboardPresentationSpec.selectedCandidateIndex(KeyboardState.Qwerty, state),
        )
    }

    @Test
    fun selectedCandidateIndexReturnsIntFromState() {
        val state = KhmerRenderState(candidates = listOf("ក", "ខ"), selectedIndex = 1)
        assertEquals(
            "selectedCandidateIndex must reflect session selectedIndex",
            1 as Int?,
            KeyboardPresentationSpec.selectedCandidateIndex(KeyboardState.Qwerty, state),
        )
    }

    @Test
    fun selectedCandidateIndexZeroIsDistinctFromNull() {
        val state = KhmerRenderState(candidates = listOf("ក", "ខ"), selectedIndex = 0)
        assertEquals(
            "selectedIndex 0 must return 0, not null",
            0 as Int?,
            KeyboardPresentationSpec.selectedCandidateIndex(KeyboardState.Qwerty, state),
        )
    }

    @Test
    fun selectedCandidateIndexIsNullInSuggestCharacterEvenWhenSessionSelectsZero() {
        // Suggest Character is tap-based: the Rust session reports selectedIndex 0,
        // but Android must not highlight a default chip (parity with iOS CharPick).
        val state = KhmerRenderState(candidates = listOf("ក", "ខ"), selectedIndex = 0)
        assertNull(
            "Suggest Character must not highlight any candidate",
            KeyboardPresentationSpec.selectedCandidateIndex(KeyboardState.SuggestCharacter, state),
        )
    }

    // ── Coeng Form display label (parity with iOS CandidateDisplayText) ──────────

    @Test
    fun candidateDisplayLabelPrefixesDottedCircleForCoengForm() {
        // "្ក" = coeng sign + ka; the coeng sign is invisible on its own,
        // so the display prefixes a dotted circle (U+25CC) as a visible base.
        assertEquals(
            "Coeng Form must display with a dotted-circle base",
            "◌្ក",
            KeyboardPresentationSpec.candidateDisplayLabel("្ក"),
        )
    }

    @Test
    fun candidateDisplayLabelLeavesNonCoengCandidateUnchanged() {
        assertEquals(
            "non-coeng candidates display exactly as-is",
            "ក",
            KeyboardPresentationSpec.candidateDisplayLabel("ក"),
        )
    }

    @Test
    fun candidateDisplayLabelLeavesEmptyCandidateUnchanged() {
        assertEquals(
            "empty candidate must not gain a dotted circle",
            "",
            KeyboardPresentationSpec.candidateDisplayLabel(""),
        )
    }

    // ── Phrase Wheel (ADR-0015) ──────────────────────────────────────────────

    @Test
    fun phraseAlternativesExcludeTheSelectedPhrase() {
        val state = KhmerRenderState(
            phraseCandidates = listOf(
                KhmerPhraseCandidate("ខ្ញុំទៅសាលា"),
                KhmerPhraseCandidate("ខ្ញុំទៅសាលារៀន"),
                KhmerPhraseCandidate("ខ្ញុំទៅ"),
            ),
            selectedPhraseIndex = 1,
        )

        val alternatives = KeyboardPresentationSpec.phraseAlternatives(state)

        assertEquals("keeps original indices for tap → selectPhrase", listOf(0, 2), alternatives.map { it.index })
        assertEquals(listOf("ខ្ញុំទៅសាលា", "ខ្ញុំទៅ"), alternatives.map { it.text })
    }

    @Test
    fun oneReadingCollapsesTheCandidateRowToStripOnly() {
        val state = KhmerRenderState(
            phraseCandidates = listOf(KhmerPhraseCandidate("ខ្ញុំ")),
            preedit = "khnhom",
        )

        assertTrue("one reading → no alternatives", KeyboardPresentationSpec.phraseAlternatives(state).isEmpty())
    }

    @Test
    fun multipleReadingsReserveTheStripAndCandidateRow() {
        val state = KhmerRenderState(
            phraseCandidates = listOf(KhmerPhraseCandidate("ខ្ញុំ"), KhmerPhraseCandidate("ខ្ញំ")),
            preedit = "khnhom",
        )

        assertEquals(1, KeyboardPresentationSpec.phraseAlternatives(state).size)
    }
}
