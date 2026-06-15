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
    fun suggestCharacterModeWithCompositionDiscardsRomanText() {
        val (handler, textField) = makeHandler()
        type("nhom", into = handler)

        handler.toggleSuggestCharacter()

        assertEquals(
            "💡 with active composition must enter Suggest Character mode",
            KeyboardState.SuggestCharacter,
            handler.keyboardState,
        )
        assertEquals(
            "entering Suggest Character mode must discard active roman composition",
            "",
            textField.text,
        )

        handler.toggleSuggestCharacter()

        assertEquals(
            "second 💡 tap must return to QWERTY",
            KeyboardState.Qwerty,
            handler.keyboardState,
        )
    }

    @Test
    fun suggestCharacterModeUsesQwertyKeysToRenderCharacterCandidatesWithoutEditingText() {
        val (handler, textField) = makeHandler()

        handler.toggleSuggestCharacter()
        val textBeforeSuggestCharacter = textField.text
        var renderedCandidates: List<String> = emptyList()
        handler.onRender = { state -> renderedCandidates = state.candidates }

        handler.sendChar("k")

        assertEquals(
            "💡 must enter Suggest Character mode",
            KeyboardState.SuggestCharacter,
            handler.keyboardState,
        )
        assertEquals(
            "QWERTY keys in Suggest Character mode must not insert roman chars",
            textBeforeSuggestCharacter,
            textField.text,
        )
        assertTrue(
            "QWERTY key k in Suggest Character mode must render Khmer candidates; got $renderedCandidates",
            renderedCandidates.contains("ក"),
        )
    }

    @Test
    fun suggestCharacterKeyRenderHappensWhileInSuggestCharacterState() {
        val (handler, _) = makeHandler()
        handler.toggleSuggestCharacter()

        var stateAtRenderTime: KeyboardState? = null
        var renderedCandidates: List<String> = emptyList()
        handler.onRender = { state ->
            stateAtRenderTime = handler.keyboardState
            renderedCandidates = state.candidates
        }

        handler.sendChar("k")

        assertEquals(
            "QWERTY key render must happen while keyboardState is SuggestCharacter",
            KeyboardState.SuggestCharacter,
            stateAtRenderTime,
        )
        assertTrue(
            "Suggest Character candidates for k must include ក; got $renderedCandidates",
            renderedCandidates.contains("ក"),
        )
    }

    @Test
    fun suggestCharacterCandidateSelectionInsertsKhmerAndResetsSuggestions() {
        val (handler, textField) = makeHandler()
        handler.toggleSuggestCharacter()
        handler.sendChar("k")

        var resetCount = 0
        handler.onSuggestCharacterReset = { resetCount += 1 }

        handler.selectCandidate(0)

        assertFalse(
            "selecting a Suggest Character candidate must insert text into the text field",
            textField.text.isEmpty(),
        )
        assertTrue(
            "inserted text must be Khmer Unicode; got '${textField.text}'",
            textField.text.all { it.code in 0x1780..0x17FF },
        )
        assertEquals(
            "Suggest Character selection must reset suggestions",
            1,
            resetCount,
        )
        assertEquals(
            "Suggest Character candidate selection keeps Suggest Character mode on",
            KeyboardState.SuggestCharacter,
            handler.keyboardState,
        )
    }

    @Test
    fun backspaceDeletesRomanPreeditCharacter() {
        val (handler, textField) = makeHandler()
        type("nh", into = handler)

        handler.sendBackspace()

        assertEquals("backspace must shorten text by one char", "n", textField.text)
    }

    // ── English mode ──────────────────────────────────────────────────────────

    @Test
    fun toggleEnglishSwitchesToEnglishMode() {
        val (handler, _) = makeHandler()

        handler.toggleEnglish()

        assertEquals(
            "toggleEnglish must switch to English mode",
            KeyboardState.English,
            handler.keyboardState,
        )
    }

    @Test
    fun toggleEnglishTwiceReturnsToQwerty() {
        val (handler, _) = makeHandler()

        handler.toggleEnglish()
        handler.toggleEnglish()

        assertEquals(
            "second toggleEnglish must return to Qwerty",
            KeyboardState.Qwerty,
            handler.keyboardState,
        )
    }

    @Test
    fun typingInEnglishModeInsertsCharactersDirectlyWithoutRomanization() {
        val (handler, textField) = makeHandler()
        handler.toggleEnglish()

        type("nhom", into = handler)

        assertEquals(
            "chars in English mode must go directly to field, not be romanized",
            "nhom",
            textField.text,
        )
    }

    @Test
    fun backspaceInEnglishModeDeletesDirectly() {
        val (handler, textField) = makeHandler()
        handler.toggleEnglish()
        type("ab", into = handler)

        handler.sendBackspace()

        assertEquals(
            "backspace in English mode must delete one char from field",
            "a",
            textField.text,
        )
    }

    @Test
    fun spaceInEnglishModeInsertsSpaceDirectly() {
        val (handler, textField) = makeHandler()
        handler.toggleEnglish()
        type("hi", into = handler)

        handler.sendSpace()

        assertEquals(
            "space in English mode must insert space without Khmer commit",
            "hi ",
            textField.text,
        )
    }

    @Test
    fun returnInEnglishModeInsertsNewlineDirectly() {
        val (handler, textField) = makeHandler()
        handler.toggleEnglish()

        handler.sendReturn()

        assertEquals(
            "return in English mode must insert newline directly",
            "\n",
            textField.text,
        )
    }

    @Test
    fun enteringEnglishModeWithPendingRomanLeavesTextInField() {
        val (handler, textField) = makeHandler()
        type("in", into = handler)
        val textBeforeToggle = textField.text

        handler.toggleEnglish()

        assertEquals(
            "entering English mode must leave existing text untouched",
            textBeforeToggle,
            textField.text,
        )
        assertEquals(KeyboardState.English, handler.keyboardState)
    }

    @Test
    fun exitingEnglishModeKeepsTextInField() {
        val (handler, textField) = makeHandler()
        handler.toggleEnglish()
        type("invite", into = handler)
        val textBeforeExit = textField.text

        handler.toggleEnglish()

        assertEquals(
            "exiting English mode must leave existing text untouched",
            textBeforeExit,
            textField.text,
        )
        assertEquals(KeyboardState.Qwerty, handler.keyboardState)
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
