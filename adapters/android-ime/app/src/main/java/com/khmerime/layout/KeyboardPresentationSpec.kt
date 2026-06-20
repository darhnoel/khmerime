package com.khmerime.layout

import com.khmerime.input.KhmerRenderState
import com.khmerime.input.KeyboardState

// Which input-chrome rows are worth their height (parity with iOS KeyboardChrome):
// roman composition owns the preedit strip + candidate row; Suggest Character owns
// only the candidate row, and only while candidates exist.
enum class ChromeRows { None, CandidateOnly, StripAndCandidate }

object KeyboardPresentationSpec {
    fun suggestionCandidates(state: KhmerRenderState): List<String> = state.candidates

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
        return if (romanHint.isNotEmpty() || state.candidates.isNotEmpty()) {
            ChromeRows.StripAndCandidate
        } else {
            ChromeRows.None
        }
    }
}
