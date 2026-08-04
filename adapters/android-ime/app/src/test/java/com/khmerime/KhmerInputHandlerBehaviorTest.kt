package com.khmerime

import android.view.inputmethod.EditorInfo
import com.khmerime.input.EnterBehavior
import com.khmerime.input.KhmerDispatcher
import com.khmerime.input.KhmerInputHandler
import com.khmerime.input.KhmerImeSession
import com.khmerime.input.KhmerRenderState
import com.khmerime.input.KeyboardState
import com.khmerime.input.SynchronousDispatcher
import org.junit.Assert.*
import org.junit.Test

// KhmerInputHandlerBehaviorTest
// =============================
// Integration tests: real Rust session via JNI, in-memory TextProxy.
// Mirrors KeyboardInputHandlerTests on iOS — behavior only, never internals.
//
// Uses SynchronousDispatcher so every test stays deterministic without
// needing to await a background thread.

class KhmerInputHandlerBehaviorTest {

    private fun makeHandler(
        dispatcher: KhmerDispatcher = SynchronousDispatcher(),
    ): Pair<KhmerInputHandler, InMemoryTextProxy> {
        val textField = InMemoryTextProxy()
        val session = KhmerImeSession()
        val handler = KhmerInputHandler(textField, session, dispatcher)
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

    // ── Selection delete ───────────────────────────────────────────────────────

    @Test
    fun backspaceDeletesEntireSelectionNotOneChar() {
        val (handler, textField) = makeHandler()
        textField.insertText("hello world")
        textField.setSelection("world")   // "world" is selected

        handler.sendBackspace()

        assertEquals(
            "backspace with a selection must delete the whole selection",
            "hello ", textField.text,
        )
    }

    @Test
    fun backspaceWithoutSelectionStillDeletesOneChar() {
        val (handler, textField) = makeHandler()
        textField.insertText("abc")

        handler.sendBackspace()

        assertEquals("backspace with no selection deletes a single char", "ab", textField.text)
    }

    // ── External clear (e.g. search-box ✖) ──────────────────────────────────────

    @Test
    fun externalFieldClearResetsCompositionAndStrip() {
        val (handler, textField) = makeHandler()
        var lastCandidates: List<String> = listOf("stale")
        handler.onRender = { state -> lastCandidates = state.candidates }
        type("nh", into = handler)                       // build a composition + candidates

        // The host clears the field externally (search-box ✖ / select-all-delete);
        // the keyboard is told via a selection update, not a key.
        textField.clearExternally()
        handler.externalTextDidChange()

        assertTrue(
            "after an external field clear, the suggestion strip must be cleared; got $lastCandidates",
            lastCandidates.isEmpty(),
        )

        // And the buffer is reset: typing starts a FRESH composition, not appended
        // to the stale one.
        type("k", into = handler)
        assertEquals("roman preedit restarts fresh after external clear", "k", textField.text)
    }

    @Test
    fun externalTextDidChangeIsNoOpWhenBufferStillMatches() {
        val (handler, textField) = makeHandler()
        type("nh", into = handler)
        val before = textField.text

        handler.externalTextDidChange()                  // field still ends with our buffer

        assertEquals("no external change → field untouched", before, textField.text)
    }

    // ── Tracer bullet ──────────────────────────────────────────────────────────

    @Test
    fun typingRomanTextShowsPreeditInTextField() {
        val (handler, textField) = makeHandler()

        type("nh", into = handler)

        assertEquals("text field must reflect roman preedit speculatively", "nh", textField.text)
    }

    @Test
    fun sendCharRunsSessionWorkThroughInjectedDispatcher() {
        val recorder = RecordingDispatcher()
        val (handler, _) = makeHandler(dispatcher = recorder)

        handler.sendChar("n")

        assertEquals("sendChar must run the session call via dispatcher.onSession", 1, recorder.onSessionCalls)
        assertEquals("sendChar must run its render via dispatcher.onMain", 1, recorder.onMainCalls)
    }

    @Test
    fun returnWithoutCompositionInsertsNewline() {
        val (handler, textField) = makeHandler()

        handler.sendReturn()

        assertEquals("empty preedit + Return must insert newline", "\n", textField.text)
    }

    // ── Editor Action (Enter honors the field's Search/Go/Send action) ──────────

    @Test
    fun returnWithoutCompositionPerformsEditorActionWhenFieldDeclaresOne() {
        val (handler, textField) = makeHandler()
        handler.enterBehavior = EnterBehavior.PerformAction(EditorInfo.IME_ACTION_SEARCH)

        handler.sendReturn()

        assertEquals(
            "Enter must perform the field's Editor Action",
            EditorInfo.IME_ACTION_SEARCH,
            textField.lastEditorAction,
        )
        assertEquals("performing an action must not insert a newline/space", "", textField.text)
    }

    @Test
    fun returnWhileComposingCommitsOnly_thenSecondEnterPerformsAction() {
        val (handler, textField) = makeHandler()
        handler.enterBehavior = EnterBehavior.PerformAction(EditorInfo.IME_ACTION_SEARCH)

        type("nhom", into = handler)
        handler.sendReturn() // first Enter: confirm/commit only

        assertFalse("first Enter must commit the Khmer", textField.text.isEmpty())
        assertFalse("committed text must not be roman", textField.text.contains("nhom"))
        assertNull(
            "first Enter must NOT perform the action — that's the two-step confirm",
            textField.lastEditorAction,
        )

        handler.sendReturn() // second Enter: now perform the action

        assertEquals(
            "second Enter (buffer empty) performs the Editor Action",
            EditorInfo.IME_ACTION_SEARCH,
            textField.lastEditorAction,
        )
    }

    @Test
    fun returnInEnglishModePerformsEditorActionWhenFieldDeclaresOne() {
        val (handler, textField) = makeHandler()
        handler.enterBehavior = EnterBehavior.PerformAction(EditorInfo.IME_ACTION_SEARCH)
        handler.toggleEnglish()

        handler.sendReturn()

        assertEquals(
            "English-mode Enter must perform the Editor Action, not insert a newline",
            EditorInfo.IME_ACTION_SEARCH,
            textField.lastEditorAction,
        )
    }

    @Test
    fun returnRemovesTrailingSpaceBeforePerformingEditorAction() {
        val (handler, textField) = makeHandler()
        handler.enterBehavior = EnterBehavior.PerformAction(EditorInfo.IME_ACTION_SEARCH)

        type("nhom", into = handler)
        handler.sendSpace()
        handler.sendReturn()

        assertEquals(
            "Editor Action must fire",
            EditorInfo.IME_ACTION_SEARCH,
            textField.lastEditorAction,
        )
        assertFalse(
            "trailing space must be stripped before the action; got '${textField.text}'",
            textField.text.endsWith(" "),
        )
    }

    @Test
    fun returnSendsRealEnterKeyForSingleLineFieldWithNoAction() {
        val (handler, textField) = makeHandler()
        handler.enterBehavior = EnterBehavior.SendEnterKey

        handler.sendReturn()

        assertEquals("Enter must send a real KEYCODE_ENTER key event", 1, textField.enterKeyCount)
        assertEquals("it must not commit a literal newline (the bug)", "", textField.text)
    }

    @Test
    fun returnInEnglishModeSendsRealEnterKeyForNoActionField() {
        val (handler, textField) = makeHandler()
        handler.enterBehavior = EnterBehavior.SendEnterKey
        handler.toggleEnglish()

        handler.sendReturn()

        assertEquals("English-mode Enter must send a real Enter key event", 1, textField.enterKeyCount)
    }

    @Test
    fun returnStripsTrailingSpaceBeforeSendingEnterKey() {
        val (handler, textField) = makeHandler()
        handler.enterBehavior = EnterBehavior.SendEnterKey

        type("nhom", into = handler)
        handler.sendSpace()
        handler.sendReturn()

        assertEquals("Enter key must be sent", 1, textField.enterKeyCount)
        assertFalse("trailing space must be stripped first", textField.text.endsWith(" "))
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
    fun sendBackspaceRunsSessionWorkThroughInjectedDispatcher() {
        val recorder = RecordingDispatcher()
        val (handler, _) = makeHandler(dispatcher = recorder)
        type("n", into = handler)
        recorder.onSessionCalls = 0
        recorder.onMainCalls = 0

        handler.sendBackspace()

        assertEquals("sendBackspace must run the session call via dispatcher.onSession", 1, recorder.onSessionCalls)
        assertEquals("sendBackspace must run its render via dispatcher.onMain", 1, recorder.onMainCalls)
    }

    @Test
    fun backspaceDeletesRomanPreeditCharacter() {
        val (handler, textField) = makeHandler()
        type("nh", into = handler)

        handler.sendBackspace()

        assertEquals("backspace must shorten text by one char", "n", textField.text)
    }

    // ── Backspace hold-repeat ─────────────────────────────────────────────────

    @Test
    fun backspaceHoldFiredDeletesFromFieldWithoutSessionCall() {
        val recorder = RecordingDispatcher()
        val (handler, textField) = makeHandler(dispatcher = recorder)
        type("nh", into = handler)
        recorder.onSessionCalls = 0

        handler.backspaceHoldFired()

        assertEquals("each hold tick must delete one char from the field", "n", textField.text)
        assertEquals(
            "backspaceHoldFired must not call the session directly — backspaceHoldEnded batches it",
            0,
            recorder.onSessionCalls,
        )
    }

    @Test
    fun backspaceHoldEndedBatchesPendingHoldTicksIntoOneSessionCall() {
        val recorder = RecordingDispatcher()
        val (handler, textField) = makeHandler(dispatcher = recorder)
        type("nhom", into = handler)
        recorder.onSessionCalls = 0

        handler.backspaceHoldFired()
        handler.backspaceHoldFired()
        handler.backspaceHoldFired()
        recorder.onSessionCalls = 0
        handler.backspaceHoldEnded()

        assertEquals(
            "backspaceHoldEnded must run exactly one batched session call for all pending ticks",
            1,
            recorder.onSessionCalls,
        )
        assertEquals("n", textField.text)
    }

    @Test
    fun backspaceHoldEndedWithNoPendingTicksIsNoOp() {
        val recorder = RecordingDispatcher()
        val (handler, _) = makeHandler(dispatcher = recorder)

        handler.backspaceHoldEnded()

        assertEquals("no pending ticks must mean no session call at all", 0, recorder.onSessionCalls)
    }

    // ── Segment editing ───────────────────────────────────────────────────────

    @Test
    fun focusSegmentOnMultiSegmentPhraseTriggersRender() {
        val (handler, _) = makeHandler()
        type("khnhomtov", into = handler)
        var renderCount = 0
        handler.onRender = { _ -> renderCount++ }

        handler.focusSegment(0)

        assertEquals("focusSegment must call onRender", 1, renderCount)
    }

    @Test
    fun focusSegmentOutOfRangeIsNoOp() {
        val (handler, _) = makeHandler()
        type("khnhomtov", into = handler)
        var renderCount = 0
        handler.onRender = { _ -> renderCount++ }

        handler.focusSegment(99)

        assertEquals("out-of-range focusSegment must not trigger render", 0, renderCount)
    }

    @Test
    fun focusSegmentTwiceOnSameIndexEntersEditMode() {
        val (handler, _) = makeHandler()
        type("khnhomtov", into = handler)
        var lastRendered: KhmerRenderState? = null
        handler.onRender = { state -> lastRendered = state }

        handler.focusSegment(0)
        handler.focusSegment(0)

        assertTrue(
            "calling focusSegment twice on same index must produce segmentEditActive=true at some point",
            lastRendered?.segmentEditActive == true,
        )
    }

    @Test
    fun focusSegmentNavigatesBetweenSegmentsWithoutEnteringEditMode() {
        val (handler, _) = makeHandler()
        type("khnhomtov", into = handler)
        handler.focusSegment(0)
        var lastRendered: KhmerRenderState? = null
        handler.onRender = { state -> lastRendered = state }

        handler.focusSegment(1)

        assertNotNull("focusSegment(1) must trigger render", lastRendered)
        assertFalse(
            "navigating to a different segment must not enter edit mode",
            lastRendered?.segmentEditActive == true,
        )
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

    // ── Preedit word tap (roman) ──────────────────────────────────────────────
    // A single word has no segments, so tapping the word shown in the preedit
    // commits the selected candidate directly (ADR-0012). The Suggestion Bar
    // stays select-only, so a different spelling can be chosen first.

    @Test
    fun tappingSingleWordInPreeditCommitsKhmer() {
        val (handler, textField) = makeHandler()
        type("nhom", into = handler)

        handler.focusSegment(0)

        assertFalse("single-word preedit tap must commit", textField.text.isEmpty())
        assertFalse("committed text must not stay roman", textField.text.contains("nhom"))
        assertTrue(
            "preedit tap must commit Khmer; got '${textField.text}'",
            textField.text.all { it.code in 0x1780..0x17FF },
        )
    }

    @Test
    fun suggestionBarTapOnSingleWordSelectsWithoutCommitting() {
        val (handler, textField) = makeHandler()
        type("nhom", into = handler)

        handler.selectCandidate(1)

        assertEquals(
            "a single-word Suggestion Bar tap must only select, not commit",
            "nhom",
            textField.text,
        )
    }
}
