package com.khmerime

enum class KeyboardLayer {
    Qwerty,
    Numeric,
    Symbols,
}

enum class KeyboardKeyAction {
    Insert,
    NextKeyboard,
    TogglePanel,
    ToggleEnglish,
    Backspace,
    Space,
    Return,
    SwitchToQwerty,
    SwitchToNumeric,
    SwitchToSymbols,
}

data class KeyboardKey(
    val label: String,
    val action: KeyboardKeyAction,
    val input: String = label,
)

object KeyboardLayerSpec {
    fun rows(layer: KeyboardLayer): List<List<KeyboardKey>> = when (layer) {
        KeyboardLayer.Qwerty -> listOf(
            letters("qwertyuiop"),
            letters("asdfghjkl"),
            listOf(special("✦", KeyboardKeyAction.TogglePanel)) +
                letters("zxcvbnm") +
                special("⌫", KeyboardKeyAction.Backspace),
            listOf(
                special("En", KeyboardKeyAction.ToggleEnglish),
                special("123", KeyboardKeyAction.SwitchToNumeric),
                special("space", KeyboardKeyAction.Space),
                KeyboardKey(".", KeyboardKeyAction.Insert),
                special("↵", KeyboardKeyAction.Return),
            ),
        )

        KeyboardLayer.Numeric -> listOf(
            inserts(listOf("1", "2", "3", "4", "5", "6", "7", "8", "9", "0")),
            inserts(listOf("-", "/", ":", ";", "(", ")", "¥", "&", "@", "\"")),
            listOf(special("#+=", KeyboardKeyAction.SwitchToSymbols)) +
                inserts(listOf(".", ",", "?", "!", "'")) +
                special("⌫", KeyboardKeyAction.Backspace),
            listOf(
                special("En", KeyboardKeyAction.ToggleEnglish),
                special("ABC", KeyboardKeyAction.SwitchToQwerty),
                special("space", KeyboardKeyAction.Space),
                special("↵", KeyboardKeyAction.Return),
            ),
        )

        KeyboardLayer.Symbols -> listOf(
            inserts(listOf("[", "]", "{", "}", "#", "%", "^", "*", "+", "=")),
            inserts(listOf("_", "\\", "|", "~", "<", ">", "€", "£", "¥", "•")),
            listOf(special("123", KeyboardKeyAction.SwitchToNumeric)) +
                inserts(listOf(".", ",", "?", "!", "'")) +
                special("⌫", KeyboardKeyAction.Backspace),
            listOf(
                special("En", KeyboardKeyAction.ToggleEnglish),
                special("ABC", KeyboardKeyAction.SwitchToQwerty),
                special("space", KeyboardKeyAction.Space),
                special("↵", KeyboardKeyAction.Return),
            ),
        )
    }

    private fun letters(keys: String): List<KeyboardKey> =
        keys.map { KeyboardKey(it.uppercase(), KeyboardKeyAction.Insert, it.toString()) }

    private fun inserts(keys: List<String>): List<KeyboardKey> =
        keys.map { KeyboardKey(it, KeyboardKeyAction.Insert) }

    private fun special(label: String, action: KeyboardKeyAction): KeyboardKey =
        KeyboardKey(label, action)
}
