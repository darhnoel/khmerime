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
