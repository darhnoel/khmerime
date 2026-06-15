package com.khmerime

import com.khmerime.layout.KeyboardKey
import com.khmerime.layout.KeyboardKeyAction
import com.khmerime.layout.KeyboardLayer
import com.khmerime.layout.KeyboardLayerSpec
import org.junit.Assert.assertEquals
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
            KeyboardLayerSpec.rows(KeyboardLayer.Qwerty).map { row -> row.map { it.label } },
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
            KeyboardLayerSpec.rows(KeyboardLayer.Numeric)[3].map { it.label },
        )
        assertEquals(
            listOf("123", ".", ",", "?", "!", "'", "⌫"),
            KeyboardLayerSpec.rows(KeyboardLayer.Symbols)[2].map { it.label },
        )
        assertEquals(
            listOf("En", "ABC", "space", "↵"),
            KeyboardLayerSpec.rows(KeyboardLayer.Symbols)[3].map { it.label },
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
