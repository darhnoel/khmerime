package com.khmerime

object KeyboardPresentationSpec {
    fun suggestionCandidates(state: KhmerRenderState): List<String> = state.candidates

    fun preeditText(keyboardState: KeyboardState?, state: KhmerRenderState): String =
        if (keyboardState == KeyboardState.SuggestCharacter) "" else state.preedit

    fun renderStateReplacesKeyboardLayer(keyboardState: KeyboardState?): Boolean =
        false

    fun selectedCandidateIndex(state: KhmerRenderState): Int? = state.selectedIndex

    fun keyboardLayerForState(keyboardState: KeyboardState?): KeyboardLayer = when (keyboardState) {
        KeyboardState.SuggestCharacter,
        KeyboardState.Qwerty,
        null -> KeyboardLayer.Qwerty
        KeyboardState.Panel -> KeyboardLayer.Qwerty
    }
}
