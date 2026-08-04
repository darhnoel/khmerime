package com.khmerime.input

// KhmerInputHandler
// =================
// Pure Kotlin input logic: every key tap goes through the Rust session and the
// render state drives all text-proxy operations. Mirrors iOS KeyboardInputHandler.
//
// Roman buffer pattern (same as iOS):
//   1. Each letter is speculatively inserted into the proxy as roman text.
//   2. On commit (Enter / Space): delete roman chars one-by-one, insert Khmer.
//   3. romanBuffer tracks how many speculative chars are in the field so we
//      know how many deleteBackward() calls to make.

class KhmerInputHandler(
    private val proxy: TextProxy,
    private val session: KhmerImeSession,
    private val dispatcher: KhmerDispatcher = QueuedDispatcher(),
) {

    private var romanBuffer = ""
    private var trailingSpace = false
    private var lastState: KhmerRenderState? = null

    // Debounced, off-hot-path model refine (Smart mode only). Renders through the
    // same `render` path so a refined result updates the strip/preview.
    private val modelRefiner = ModelRefiner(session, dispatcher) { state -> render(state) }

    // Counts how many roman-buffer chars were deleted by backspaceHoldFired()
    // without a matching session call. backspaceHoldEnded() drains this with
    // one batched session block, then resets to 0.
    private var pendingHoldBackspaces = 0

    var keyboardState: KeyboardState = KeyboardState.Qwerty
        private set

    // What Enter does in the current host field, set from the field's Editor
    // Action by KhmerInputMethodService. Defaults to a plain newline so fields
    // with no action (and tests) behave as before. See CONTEXT.md "Editor Action".
    var enterBehavior: EnterBehavior = EnterBehavior.Newline

    var onRender: ((KhmerRenderState) -> Unit)? = null
    var onTransition: ((KeyboardState) -> Unit)? = null
    var onSuggestCharacterReset: (() -> Unit)? = null

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    fun focusIn() {
        render(session.focusIn())
    }

    fun focusOut() {
        modelRefiner.cancel()
        session.focusOut()
    }

    // Called by the service on onUpdateSelection — an external/host change to the
    // field (search-box ✖, select-all + delete, tap elsewhere). If we have a live
    // roman buffer but the text before the cursor no longer ends with it, the
    // field was cleared/changed outside the keyboard: reset the composition and
    // clear the suggestion strip. Mirrors iOS KeyboardInputHandler.textDidChange.
    fun externalTextDidChange() {
        if (romanBuffer.isEmpty()) return
        // Our speculative roman is inserted at the cursor, so if the field wasn't
        // changed externally the text before the cursor still ends with it.
        val before = proxy.textBeforeCursor ?: ""
        if (before.endsWith(romanBuffer)) return
        // External clear/change: reset composition + strip.
        romanBuffer = ""
        modelRefiner.cancel()
        dispatcher.onSession {
            session.processEnter()               // flush/reset the session composition
            dispatcher.onMain {
                render(KhmerRenderState())        // empty strip: no candidates/preedit/segments
                if (keyboardState == KeyboardState.SuggestCharacter) transitionTo(KeyboardState.Qwerty)
            }
        }
    }

    // ── Key actions ───────────────────────────────────────────────────────────

    fun sendChar(ch: String) {
        if (keyboardState == KeyboardState.English) {
            proxy.insertText(ch)
            return
        }
        if (keyboardState == KeyboardState.SuggestCharacter) {
            dispatcher.onSession {
                val state = session.processCharacter(ch)
                dispatcher.onMain { render(state) }
            }
            return
        }
        if (keyboardState == KeyboardState.Panel) transitionTo(KeyboardState.Qwerty)
        trailingSpace = false
        proxy.insertText(ch)
        romanBuffer += ch
        dispatcher.onSession {
            val state = session.processCharacter(ch)
            dispatcher.onMain {
                val committed = state.commitText
                if (committed != null && committed.isNotEmpty()) {
                    proxy.deleteBackward(romanBuffer.length)
                    proxy.insertText(committed)
                    romanBuffer = ""
                }
                render(state)
                // Smart mode: schedule a debounced model refine of the live composition,
                // off this keystroke's hot path. No-op in Standard (no visible refiner).
                if (romanBuffer.isNotEmpty() && session.isModelMode()) modelRefiner.schedule(romanBuffer)
            }
        }
    }

    fun sendBackspace() {
        // A selection deletes as a whole, regardless of mode. deleteSurroundingText
        // (what deleteBackward uses) does not touch a selection, so bulk-delete of
        // selected text was a no-op before this. Also drop any speculative roman
        // buffer, since the field state we tracked is gone.
        if (proxy.selectedText != null) {
            proxy.deleteSelection()
            if (romanBuffer.isNotEmpty()) {
                romanBuffer = ""
                dispatcher.onSession {
                    val state = session.processBackspace()
                    dispatcher.onMain { render(state) }
                }
            }
            return
        }
        if (keyboardState == KeyboardState.English) {
            proxy.deleteBackward()
            return
        }
        trailingSpace = false
        if (keyboardState == KeyboardState.SuggestCharacter) {
            val current = lastState
            if (current != null && current.candidates.isNotEmpty()) {
                dispatcher.onSession {
                    session.enterCharPick()
                    dispatcher.onMain {
                        lastState = null
                        onSuggestCharacterReset?.invoke()
                        transitionTo(KeyboardState.SuggestCharacter)
                    }
                }
            } else {
                proxy.deleteBackward()
            }
            return
        }
        if (romanBuffer.isNotEmpty()) romanBuffer = romanBuffer.dropLast(1)
        proxy.deleteBackward()
        dispatcher.onSession {
            val state = session.processBackspace()
            dispatcher.onMain { render(state) }
        }
    }

    fun backspaceHoldFired() {
        if (keyboardState == KeyboardState.English) {
            proxy.deleteBackward()
            return
        }
        trailingSpace = false
        if (keyboardState == KeyboardState.SuggestCharacter) {
            proxy.deleteBackward()
            return
        }
        if (romanBuffer.isNotEmpty()) {
            romanBuffer = romanBuffer.dropLast(1)
            pendingHoldBackspaces++
        }
        proxy.deleteBackward()
        // No session dispatch — backspaceHoldEnded() batches them all at once.
    }

    fun backspaceHoldEnded() {
        val count = pendingHoldBackspaces
        pendingHoldBackspaces = 0
        if (count <= 0) return
        dispatcher.onSession {
            var state: KhmerRenderState? = null
            repeat(count) { state = session.processBackspace() }
            val finalState = state ?: return@onSession
            dispatcher.onMain { render(finalState) }
        }
    }

    fun sendSpace() {
        if (keyboardState == KeyboardState.English) {
            proxy.insertText(" ")
            return
        }
        commitComposition()
        proxy.insertText(" ")
        trailingSpace = true
    }

    fun sendReturn() {
        if (keyboardState == KeyboardState.English) {
            // Passthrough mode still honors the field's Editor Action, so Enter
            // searches/sends rather than dropping a newline (which a single-line
            // field renders as a space — the Google Search bug).
            performEnterTerminal()
            return
        }
        if (keyboardState == KeyboardState.SuggestCharacter) {
            selectCandidate(0)
            return
        }
        if (romanBuffer.isNotEmpty()) {
            // Two-step: Enter while composing only COMMITS the Khmer (the confirm
            // step). The field's action / newline happens on the next Enter, once
            // the buffer is empty — so a search isn't fired on an unconfirmed word.
            commitComposition()
            return
        }
        performEnterTerminal()
    }

    // Enter with nothing left to commit: perform the field's Editor Action, or
    // insert a newline when the field has none / is multiline. Strips an
    // auto-inserted trailing space first so a search query has no stray space.
    private fun performEnterTerminal() {
        if (trailingSpace) {
            proxy.deleteBackward()
            trailingSpace = false
        }
        when (val behavior = enterBehavior) {
            is EnterBehavior.PerformAction -> proxy.performEditorAction(behavior.actionId)
            EnterBehavior.SendEnterKey -> proxy.sendEnterKey()
            EnterBehavior.Newline -> proxy.insertText("\n")
        }
    }

    fun toggleSuggestCharacter() {
        when (keyboardState) {
            KeyboardState.Panel -> transitionTo(KeyboardState.Qwerty)
            KeyboardState.SuggestCharacter -> {
                session.exitCharPick()
                lastState = null
                transitionTo(KeyboardState.Qwerty)
            }
            KeyboardState.Qwerty,
            KeyboardState.English -> {
                repeat(romanBuffer.length) { proxy.deleteBackward() }
                romanBuffer = ""
                trailingSpace = false
                session.enterCharPick()
                lastState = null
                transitionTo(KeyboardState.SuggestCharacter)
                onSuggestCharacterReset?.invoke()
            }
        }
    }

    fun togglePanel() = toggleSuggestCharacter()

    fun focusSegment(index: Int) {
        val current = lastState ?: return
        val segments = current.segments
        if (segments.isEmpty()) {
            // Single word: the preedit word is tappable and commits the shown
            // (selected) candidate directly — mirrors iOS chipTapped (ADR-0012).
            commitComposition()
            return
        }
        if (index < 0 || index >= segments.size) return
        val focusedIndex = current.focusedSegmentIndex ?: 0
        if (index == focusedIndex && !current.segmentEditActive) {
            render(session.processTab())
            transitionTo(KeyboardState.Qwerty)
        } else if (index != focusedIndex) {
            val diff = index - focusedIndex
            var state = current
            if (diff > 0) repeat(diff) { state = session.processRight() }
            else repeat(-diff) { state = session.processLeft() }
            render(state)
        }
    }

    fun toggleEnglish() {
        when (keyboardState) {
            KeyboardState.English -> transitionTo(KeyboardState.Qwerty)
            else -> {
                session.processEnter()
                romanBuffer = ""
                trailingSpace = false
                transitionTo(KeyboardState.English)
            }
        }
    }

    fun selectCandidate(index: Int) {
        if (keyboardState == KeyboardState.SuggestCharacter) {
            lastState?.candidates?.getOrNull(index)?.let { proxy.insertText(it) }
            session.enterCharPick()
            lastState = null
            onSuggestCharacterReset?.invoke()
            transitionTo(KeyboardState.SuggestCharacter)
            return
        }
        render(session.processDigit(index + 1))
    }

    // Tapping a Phrase Wheel card selects that whole-phrase reading (ADR-0015): it
    // becomes the strip's preview; Space/Enter then commit it. Tapping never commits.
    fun selectPhrase(index: Int) {
        render(session.selectPhrase(index))
    }

    // ── Private ───────────────────────────────────────────────────────────────

    private fun commitComposition() {
        if (keyboardState == KeyboardState.SuggestCharacter) return
        modelRefiner.cancel()
        val state = session.processEnter()
        val khmer = if (state.segments.isEmpty()) {
            state.commitText ?: ""
        } else {
            state.segments.joinToString(separator = "") { it.output }
        }
        repeat(romanBuffer.length) { proxy.deleteBackward() }
        if (khmer.isNotEmpty()) proxy.insertText(khmer)
        romanBuffer = ""
        trailingSpace = false
        render(state)
        if (keyboardState == KeyboardState.Panel) transitionTo(KeyboardState.Qwerty)
    }

    private fun render(state: KhmerRenderState) {
        lastState = state
        onRender?.invoke(state)
    }

    private fun transitionTo(state: KeyboardState) {
        if (keyboardState == state) return
        keyboardState = state
        onTransition?.invoke(state)
    }
}
