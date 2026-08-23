package com.khmerime

import com.khmerime.layout.KeyboardKey
import com.khmerime.layout.KeyboardKeyAction
import com.khmerime.layout.KeyboardLayer
import com.khmerime.layout.KeyboardLayerSpec
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Test

class KeyboardLayerSpecTest {

    private fun allKeys(layer: KeyboardLayer) = KeyboardLayerSpec.rows(layer).flatten()

    @Test
    fun qwertyLayerMatchesKeyboardRows() {
        assertEquals(
            listOf(
                listOf("Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"),
                listOf("A", "S", "D", "F", "G", "H", "J", "K", "L"),
                listOf("✦", "Z", "X", "C", "V", "B", "N", "M", "⌫"),
                listOf("En", "123", "space", ".", "↵"),
            ),
            // the globe is icon-rendered, so it carries no text label; compare
            // only the labelled keys.
            KeyboardLayerSpec.rows(KeyboardLayer.Qwerty)
                .map { row -> row.filter { it.iconRes == null }.map { it.label } },
        )
    }

    @Test
    fun numericAndSymbolsLayersMatchModeSwitchRows() {
        assertEquals(
            listOf("#+=", ".", ",", "?", "!", "'", "⌫"),
            KeyboardLayerSpec.rows(KeyboardLayer.Numeric)[2].map { it.label },
        )
        assertEquals(
            listOf("En", "ABC", "space", "↵"),
            KeyboardLayerSpec.rows(KeyboardLayer.Numeric)[3].filter { it.iconRes == null }.map { it.label },
        )
        assertEquals(
            listOf("123", ".", ",", "?", "!", "'", "⌫"),
            KeyboardLayerSpec.rows(KeyboardLayer.Symbols)[2].map { it.label },
        )
        assertEquals(
            listOf("En", "ABC", "space", "↵"),
            KeyboardLayerSpec.rows(KeyboardLayer.Symbols)[3].filter { it.iconRes == null }.map { it.label },
        )
    }

    @Test
    fun everyLayerHasANextKeyboardGlobeFirstInTheBottomRow() {
        // App Store / user-reported parity (ADR-0022): a next-keyboard control
        // is always present. On Android it is an icon globe at the start of the
        // bottom row (a monochrome vector drawable, not a color emoji).
        KeyboardLayer.entries.forEach { layer ->
            val globe = KeyboardLayerSpec.rows(layer).last().first()
            assertEquals(
                "$layer globe must switch keyboards",
                KeyboardKeyAction.NextKeyboard,
                globe.action,
            )
            assertNotNull("$layer globe must render an icon, not text", globe.iconRes)
        }
    }

    @Test
    fun visibleDigitsPunctuationAndSymbolsAreLiteralKeycaps() {
        val numericLiterals = allKeys(KeyboardLayer.Numeric).filter { it.action == KeyboardKeyAction.InsertLiteral }
        val symbolLiterals = allKeys(KeyboardLayer.Symbols).filter { it.action == KeyboardKeyAction.InsertLiteral }

        assertEquals("all 25 numeric-layer text keys must be literal", 25, numericLiterals.size)
        assertEquals("all 25 symbol-layer text keys must be literal", 25, symbolLiterals.size)
        assertEquals(
            KeyboardKeyAction.InsertLiteral,
            allKeys(KeyboardLayer.Qwerty).single { it.label == "." }.action,
        )
        assertEquals(
            KeyboardKeyAction.Insert,
            allKeys(KeyboardLayer.Qwerty).single { it.label == "O" }.action,
        )
    }

    @Test
    fun charPickKeyUsesSparkle() {
        val key = allKeys(KeyboardLayer.Qwerty).find { it.action == KeyboardKeyAction.TogglePanel }
        assertEquals("CharPick toggle must use ✦", "✦", key?.label)
    }

    @Test
    fun englishToggleKeyUsesEn() {
        KeyboardLayer.entries.forEach { layer ->
            val key = allKeys(layer).find { it.action == KeyboardKeyAction.ToggleEnglish }
            assertEquals("English toggle key in $layer must use En", "En", key?.label)
        }
    }

    @Test
    fun returnKeyUsesArrow() {
        KeyboardLayer.entries.forEach { layer ->
            val key = allKeys(layer).find { it.action == KeyboardKeyAction.Return }
            assertEquals("Return key in $layer must use ↵", "↵", key?.label)
        }
    }

    @Test
    fun backspaceKeyKeepsEraseSymbol() {
        KeyboardLayer.entries.forEach { layer ->
            val key = allKeys(layer).find { it.action == KeyboardKeyAction.Backspace }
            assertEquals("Backspace in $layer must stay ⌫", "⌫", key?.label)
        }
    }
}
