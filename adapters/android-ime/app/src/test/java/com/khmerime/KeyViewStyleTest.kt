package com.khmerime

import com.khmerime.layout.KeyboardKey
import com.khmerime.layout.KeyboardKeyAction
import com.khmerime.layout.KeyViewStyle
import org.junit.Assert.*
import org.junit.Test

class KeyViewStyleTest {

    @Test
    fun insertKeyHasBaseWeight() {
        val key = KeyboardKey("A", KeyboardKeyAction.Insert, "a")
        assertEquals(1f, KeyViewStyle.weightFor(key), 0f)
    }

    @Test
    fun spaceKeyHasHigherWeightThanInsertKey() {
        val space = KeyboardKey("space", KeyboardKeyAction.Space)
        val insert = KeyboardKey("A", KeyboardKeyAction.Insert, "a")
        assertTrue(
            "space must outweigh a letter key",
            KeyViewStyle.weightFor(space) > KeyViewStyle.weightFor(insert),
        )
    }

    @Test
    fun returnKeyHasHigherWeightThanInsertKey() {
        val ret = KeyboardKey("⏎", KeyboardKeyAction.Return)
        val insert = KeyboardKey("A", KeyboardKeyAction.Insert, "a")
        assertTrue(
            "return must outweigh a letter key",
            KeyViewStyle.weightFor(ret) > KeyViewStyle.weightFor(insert),
        )
    }

    @Test
    fun insertKeyIsNotAnActionKey() {
        val key = KeyboardKey("A", KeyboardKeyAction.Insert, "a")
        assertFalse("Insert is not an action key", KeyViewStyle.isAction(key))
    }

    @Test
    fun backspaceIsAnActionKey() {
        val key = KeyboardKey("⌫", KeyboardKeyAction.Backspace)
        assertTrue("Backspace is an action key", KeyViewStyle.isAction(key))
    }

    @Test
    fun spaceIsAnActionKey() {
        val key = KeyboardKey("space", KeyboardKeyAction.Space)
        assertTrue("Space is an action key", KeyViewStyle.isAction(key))
    }

    @Test
    fun togglePanelIsAnActionKey() {
        val key = KeyboardKey("💡", KeyboardKeyAction.TogglePanel)
        assertTrue("TogglePanel is an action key", KeyViewStyle.isAction(key))
    }

    @Test
    fun englishToggleIsAnActionKey() {
        val key = KeyboardKey("En", KeyboardKeyAction.ToggleEnglish)
        assertTrue("ToggleEnglish is an action key", KeyViewStyle.isAction(key))
    }

    @Test
    fun englishToggleHasSameWeightAsOtherActionKeys() {
        val en = KeyboardKey("En", KeyboardKeyAction.ToggleEnglish)
        val backspace = KeyboardKey("⌫", KeyboardKeyAction.Backspace)
        assertEquals(
            "ToggleEnglish must have same weight as Backspace",
            KeyViewStyle.weightFor(backspace),
            KeyViewStyle.weightFor(en),
            0f,
        )
    }
}
