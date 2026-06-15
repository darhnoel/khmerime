package com.khmerime

import org.junit.Assert.assertEquals
import org.junit.Test

class KeyboardLayerSpecTest {
    @Test
    fun qwertyLayerMatchesIosKeyboardRows() {
        assertEquals(
            listOf(
                listOf("Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"),
                listOf("A", "S", "D", "F", "G", "H", "J", "K", "L"),
                listOf("💡", "Z", "X", "C", "V", "B", "N", "M", "⌫"),
                listOf("🌐", "123", "space", ".", "⏎"),
            ),
            KeyboardLayerSpec.rows(KeyboardLayer.Qwerty).map { row -> row.map { it.label } },
        )
    }

    @Test
    fun numericAndSymbolsLayersMatchIosModeSwitchRows() {
        assertEquals(
            listOf("#+=", ".", ",", "?", "!", "'", "⌫"),
            KeyboardLayerSpec.rows(KeyboardLayer.Numeric)[2].map { it.label },
        )
        assertEquals(
            listOf("🌐", "ABC", "space", "⏎"),
            KeyboardLayerSpec.rows(KeyboardLayer.Numeric)[3].map { it.label },
        )
        assertEquals(
            listOf("123", ".", ",", "?", "!", "'", "⌫"),
            KeyboardLayerSpec.rows(KeyboardLayer.Symbols)[2].map { it.label },
        )
        assertEquals(
            listOf("🌐", "ABC", "space", "⏎"),
            KeyboardLayerSpec.rows(KeyboardLayer.Symbols)[3].map { it.label },
        )
    }
}
