package com.khmerime.layout

import com.khmerime.input.KhmerRenderState
import com.khmerime.input.KeyboardState

// Which input-chrome rows are worth their height (parity with iOS KeyboardChrome):
// roman composition owns the preedit strip + candidate row; Suggest Character owns
// only the candidate row, and only while candidates exist.
enum class ChromeRows { None, CandidateOnly, StripAndCandidate, StripOnly }

// One selectable card in the Phrase Wheel: the phrase text plus its index into the
// render state's phraseCandidates (so a tap maps to selectPhrase).
data class PhraseAlternative(val index: Int, val text: String)

object KeyboardPresentationSpec {
    fun suggestionCandidates(state: KhmerRenderState): List<String> = state.candidates

    // The Phrase Wheel cards (ADR-0015): whole-phrase alternatives EXCLUDING the one the
    // strip previews (selectedPhraseIndex). Empty → the strip stands alone.
    fun phraseAlternatives(state: KhmerRenderState): List<PhraseAlternative> =
        state.phraseCandidates.mapIndexedNotNull { index, candidate ->
            if (index == state.selectedPhraseIndex) null else PhraseAlternative(index, candidate.text)
        }

    // Composition shows the Phrase Wheel in the chip row; CharPick and Segment Edit show
    // word candidates instead.
    fun showsPhraseWheel(keyboardState: KeyboardState?, state: KhmerRenderState): Boolean =
        keyboardState != KeyboardState.SuggestCharacter &&
            !state.segmentEditActive &&
            phraseAlternatives(state).isNotEmpty()

    fun preeditText(keyboardState: KeyboardState?, state: KhmerRenderState): String =
        if (keyboardState == KeyboardState.SuggestCharacter ||
            keyboardState == KeyboardState.English) "" else state.preedit

    fun renderStateReplacesKeyboardLayer(keyboardState: KeyboardState?): Boolean =
        false

    fun isToggleActive(key: KeyboardKey, keyboardState: KeyboardState): Boolean =
        key.action == KeyboardKeyAction.TogglePanel && keyboardState == KeyboardState.SuggestCharacter

    fun romanRowText(state: KhmerRenderState, romanBuffer: String): String {
        if (state.segments.isEmpty()) return romanBuffer
        val editIndex = if (state.segmentEditActive) state.segmentEditIndex else null
        val parts = state.segments.mapIndexed { i, seg ->
            if (i == editIndex) "[${seg.input}]" else seg.input
        }
        val joined = parts.joinToString(" · ")
        return if (state.segmentEditActive) "✏ $joined" else joined
    }

    fun segmentKhmerTexts(state: KhmerRenderState): List<String> =
        state.segments.map { it.output }

    fun focusedSegmentIndex(state: KhmerRenderState): Int? =
        if (state.segments.isEmpty()) null else state.focusedSegmentIndex

    fun isKeyActive(key: KeyboardKey, keyboardState: KeyboardState): Boolean = when (key.action) {
        KeyboardKeyAction.TogglePanel -> keyboardState == KeyboardState.SuggestCharacter
        KeyboardKeyAction.ToggleEnglish -> keyboardState == KeyboardState.English
        else -> false
    }

    // Suggest Character is tap-based: suppress the session's default selection so
    // no chip is highlighted (parity with iOS CharPick). Other modes pass through.
    fun selectedCandidateIndex(keyboardState: KeyboardState?, state: KhmerRenderState): Int? =
        if (keyboardState == KeyboardState.SuggestCharacter) null else state.selectedIndex

    fun keyboardLayerForState(keyboardState: KeyboardState?): KeyboardLayer = when (keyboardState) {
        KeyboardState.SuggestCharacter,
        KeyboardState.Qwerty,
        KeyboardState.English,
        null -> KeyboardLayer.Qwerty
        KeyboardState.Panel -> KeyboardLayer.Qwerty
    }

    // Display-only: a candidate beginning with the Khmer coeng sign (U+17D2) is
    // an invisible subscript joiner, so prefix a dotted circle (U+25CC) to make
    // the Coeng Form legible on the chip. Insertion still uses the raw candidate
    // (selection is by index). Mirrors iOS CandidateDisplayText.
    fun candidateDisplayLabel(candidate: String): String =
        if (candidate.firstOrNull() == COENG_SIGN) "$DOTTED_CIRCLE$candidate" else candidate

    private const val COENG_SIGN = '្'
    private const val DOTTED_CIRCLE = "◌"

    // Mirrors iOS KeyboardChrome.rows: Suggest Character shows only the candidate
    // row and only while candidates exist; roman composition keeps strip +
    // candidate together whenever it has a hint or candidates; otherwise nothing.
    fun chromeRows(
        keyboardState: KeyboardState?,
        romanHint: String,
        state: KhmerRenderState,
    ): ChromeRows {
        if (keyboardState == KeyboardState.SuggestCharacter) {
            return if (state.candidates.isEmpty()) ChromeRows.None else ChromeRows.CandidateOnly
        }
        // The candidate row is the Phrase Wheel (alternatives) during normal composition,
        // or the word candidate row while editing a segment. When neither has anything to
        // show, the strip stands alone (StripOnly) so the row collapses (ADR-0015).
        val candidateRowVisible =
            if (state.segmentEditActive) state.candidates.isNotEmpty()
            else phraseAlternatives(state).isNotEmpty()
        return when {
            candidateRowVisible -> ChromeRows.StripAndCandidate
            romanHint.isNotEmpty() -> ChromeRows.StripOnly
            else -> ChromeRows.None
        }
    }
}
