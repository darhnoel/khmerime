package com.khmerime.layout

import com.khmerime.R

enum class KeyboardLayer {
    Qwerty,
    Numeric,
    Symbols,
}

enum class KeyboardKeyAction {
    Insert,
    InsertLiteral,
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
    // When set, the key renders this drawable instead of its text label (used
    // for the monochrome globe on the next-keyboard key). `label` is kept as the
    // accessibility / key-preview text.
    val iconRes: Int? = null,
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
                // globe stays in the standard left position; En moves to the old
                // "." slot (right of space) so the left side isn't crowded.
                globeKey(),
                special("123", KeyboardKeyAction.SwitchToNumeric),
                special("space", KeyboardKeyAction.Space),
                special("En", KeyboardKeyAction.ToggleEnglish),
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
                globeKey(),
                special("ABC", KeyboardKeyAction.SwitchToQwerty),
                special("space", KeyboardKeyAction.Space),
                special("En", KeyboardKeyAction.ToggleEnglish),
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
                globeKey(),
                special("ABC", KeyboardKeyAction.SwitchToQwerty),
                special("space", KeyboardKeyAction.Space),
                special("En", KeyboardKeyAction.ToggleEnglish),
                special("↵", KeyboardKeyAction.Return),
            ),
        )
    }

    private fun letters(keys: String): List<KeyboardKey> =
        keys.map { KeyboardKey(it.uppercase(), KeyboardKeyAction.Insert, it.toString()) }

    private fun inserts(keys: List<String>): List<KeyboardKey> =
        keys.map { KeyboardKey(it, KeyboardKeyAction.InsertLiteral) }

    private fun special(label: String, action: KeyboardKeyAction): KeyboardKey =
        KeyboardKey(label, action)

    // The next-keyboard key: a monochrome globe vector (ADR-0022). "Globe" is
    // the key-preview / accessibility label; the icon is what's drawn.
    private fun globeKey(): KeyboardKey =
        KeyboardKey("Globe", KeyboardKeyAction.NextKeyboard, iconRes = R.drawable.ic_tip_globe)
}
