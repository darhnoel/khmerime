package com.khmerime.layout

object KeyViewStyle {
    fun weightFor(key: KeyboardKey): Float = when (key.action) {
        KeyboardKeyAction.Space -> 4f
        KeyboardKeyAction.Return -> 1.4f
        KeyboardKeyAction.TogglePanel,
        KeyboardKeyAction.ToggleEnglish,
        KeyboardKeyAction.Backspace,
        KeyboardKeyAction.NextKeyboard,
        KeyboardKeyAction.SwitchToQwerty,
        KeyboardKeyAction.SwitchToNumeric,
        KeyboardKeyAction.SwitchToSymbols -> 1.3f
        KeyboardKeyAction.Insert -> 1f
    }

    fun isAction(key: KeyboardKey): Boolean = key.action != KeyboardKeyAction.Insert
}
