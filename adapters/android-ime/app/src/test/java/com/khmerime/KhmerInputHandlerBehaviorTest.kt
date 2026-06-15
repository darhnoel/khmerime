package com.khmerime

import org.junit.Assert.*
import org.junit.Test

// KhmerInputHandlerBehaviorTest
// =============================
// Integration tests: real Rust session via JNI, in-memory TextProxy.
// Mirrors KeyboardInputHandlerTests on iOS — behavior only, never internals.

class KhmerInputHandlerBehaviorTest {

    private fun makeHandler(): Pair<KhmerInputHandler, InMemoryTextProxy> {
        val textField = InMemoryTextProxy()
        val session = KhmerImeSession()
        val handler = KhmerInputHandler(textField, session)
        handler.focusIn()
        return Pair(handler, textField)
    }

    private fun type(word: String, into: KhmerInputHandler) {
        for (ch in word) into.sendChar(ch.toString())
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    @Test
    fun focusLifecycleDoesNotEditTextField() {
        val (handler, textField) = makeHandler()

        handler.focusOut()

        assertTrue("focusIn + focusOut must leave text empty", textField.text.isEmpty())
    }

    // ── Tracer bullet ──────────────────────────────────────────────────────────

    @Test
    fun typingRomanTextShowsPreeditInTextField() {
        val (handler, textField) = makeHandler()

        type("nh", into = handler)

        assertEquals("text field must reflect roman preedit speculatively", "nh", textField.text)
    }

    @Test
    fun returnWithoutCompositionInsertsNewline() {
        val (handler, textField) = makeHandler()

        handler.sendReturn()

        assertEquals("empty preedit + Return must insert newline", "\n", textField.text)
    }

    @Test
    fun returnAfterSpaceRemovesTrailingSpaceBeforeNewline() {
        val (handler, textField) = makeHandler()
        type("nhom", into = handler)

        handler.sendSpace()
        handler.sendReturn()

        assertTrue("return must insert newline", textField.text.endsWith("\n"))
        assertFalse(
            "trailing space must be removed before newline; got '${textField.text}'",
            textField.text.endsWith(" \n"),
        )
    }

    @Test
    fun backspaceDeletesRomanPreeditCharacter() {
        val (handler, textField) = makeHandler()
        type("nh", into = handler)

        handler.sendBackspace()

        assertEquals("backspace must shorten text by one char", "n", textField.text)
    }

    @Test
    fun returnWithCompositionCommitsKhmerText() {
        val (handler, textField) = makeHandler()
        type("nhom", into = handler)

        handler.sendReturn()

        assertFalse("text field must have committed text", textField.text.isEmpty())
        assertFalse("committed text must not be roman", textField.text.contains("nhom"))
        val isKhmer = textField.text.all { it.code in 0x1780..0x17FF }
        assertTrue(
            "committed text must be Khmer Unicode; got '${textField.text}'",
            isKhmer,
        )
    }
}
