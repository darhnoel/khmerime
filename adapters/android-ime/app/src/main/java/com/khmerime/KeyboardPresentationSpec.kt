package com.khmerime

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

    fun selectedCandidateIndex(state: KhmerRenderState): Int? = state.selectedIndex

    fun keyboardLayerForState(keyboardState: KeyboardState?): KeyboardLayer = when (keyboardState) {
        KeyboardState.SuggestCharacter,
        KeyboardState.Qwerty,
        KeyboardState.English,
        null -> KeyboardLayer.Qwerty
        KeyboardState.Panel -> KeyboardLayer.Qwerty
    }
}
