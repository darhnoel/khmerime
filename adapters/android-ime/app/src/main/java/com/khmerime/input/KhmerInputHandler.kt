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

    // Deferred path: fast typing appends roman without the expensive per-key candidate
    // decode. Instead each keystroke keeps the last suggestions and (re)schedules one decode
    // that runs once typing pauses (RECOMPUTE_DEBOUNCE_MS). See ModelRefiner for the
    // same debounce+revision guard shape.
    private val recomputer = ModelRefiner(
        session,
        dispatcher,
        debounceMs = RECOMPUTE_DEBOUNCE_MS,
        op = { s, _ -> s.recomputeNow() },
    ) { state ->
        render(state)
        // The decode has landed; in Smart mode refine it once (still off the hot path).
        if (romanBuffer.isNotEmpty() && session.isModelMode()) modelRefiner.schedule(romanBuffer)
    }

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
    // Fired when a deferred keystroke is pending its decode. Carries the live roman
    // so the UI can update it without discarding the last decoded suggestion rows.
    var onPendingDecode: ((String) -> Unit)? = null
    var onTransition: ((KeyboardState) -> Unit)? = null
    var onSuggestCharacterReset: (() -> Unit)? = null

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    fun focusIn() {
        render(session.focusIn())
    }

    fun focusOut() {
        modelRefiner.cancel()
        recomputer.cancel()
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
        recomputer.cancel()
        dispatcher.onSession {
            session.processEnter()               // flush/reset the session composition
            dispatcher.onMain {
                render(KhmerRenderState())        // empty strip: no candidates/preedit/segments
                if (keyboardState == KeyboardState.SuggestCharacter) transitionTo(KeyboardState.Qwerty)
            }
        }
    }

    // ── Key actions ───────────────────────────────────────────────────────────

    // A new physical character gesture means typing has not paused yet. Cancel a
    // decode scheduled by the previous ACTION_UP before it can render suggestion
    // rows underneath the finger that is now held down. The accepted ACTION_UP
    // will schedule a fresh decode for the latest Roman buffer.
    fun keyTouchBegan() {
        recomputer.cancel()
        modelRefiner.cancel()
    }

    // Idle-tray characters are already final Khmer glyphs. Insert them directly;
    // routing them through Roman transliteration would incorrectly start a composition.
    fun insertQuickAccess(text: String) {
        if (romanBuffer.isNotEmpty()) return
        trailingSpace = false
        proxy.insertText(text)
    }

    fun sendLiteralKeycap(text: String) {
        trailingSpace = false
        if (keyboardState == KeyboardState.English ||
            keyboardState == KeyboardState.SuggestCharacter ||
            romanBuffer.isEmpty()
        ) {
            proxy.insertText(text)
            return
        }
        commitComposition(suffix = text)
    }

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
        // A standalone single-keycap char (digit/symbol on the "123"/"#+=" layer) does not compose:
        // it auto-commits to its own Khmer form (e.g. "1" -> "១"). Inserting the raw key
        // optimistically would paint the Latin glyph, then the deferred call deletes it and inserts
        // the Khmer one — a visible flicker. Skip the optimistic insert for it; the deferred commit
        // inserts the final glyph once (deleteBackward(0) below is a no-op). Only when it is
        // standalone (romanBuffer empty) — a digit mid-composition is part of the buffer and still
        // inserts optimistically. `isSingleKeycap` mirrors Rust `is_single_keycap_char`; the two
        // must change together, and `romanBuffer.isEmpty()` mirrors its `composition_raw.len()==1`
        // auto-commit condition.
        if (!(romanBuffer.isEmpty() && isSingleKeycap(ch))) {
            proxy.insertText(ch)
            romanBuffer += ch
        }
        // Deferred path: append the roman WITHOUT the per-key candidate decode (the
        // 300–800 ms cost that made fast typing churn). Update the roman immediately and
        // let `recomputer` run the decode once typing pauses. Single-keycap auto-commit
        // (digit/symbol) still comes back from the deferred call and is applied at once.
        onPendingDecode?.invoke(romanBuffer)
        dispatcher.onSession {
            val state = session.processCharacterDeferred(ch)
            dispatcher.onMain {
                val committed = state.commitText
                if (committed != null && committed.isNotEmpty()) {
                    // For a standalone single-keycap the raw key was never optimistically inserted,
                    // so there is nothing to delete (romanBuffer is empty); only replace the
                    // optimistic roman when we actually inserted one.
                    if (romanBuffer.isNotEmpty()) proxy.deleteBackward(romanBuffer.length)
                    proxy.insertText(committed)
                    romanBuffer = ""
                    recomputer.cancel()
                    render(state)
                    return@onMain
                }
                if (romanBuffer.isNotEmpty()) recomputer.schedule(romanBuffer)
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
        // Drop any deferred decode from prior keystrokes so its stale result can't
        // land after this backspace's fresh decode.
        recomputer.cancel()
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

    private fun commitComposition(suffix: String? = null) {
        if (keyboardState == KeyboardState.SuggestCharacter) return
        modelRefiner.cancel()
        recomputer.cancel()
        val state = session.processEnter()
        val khmer = if (state.segments.isEmpty()) {
            state.commitText ?: ""
        } else {
            state.segments.joinToString(separator = "") { it.output }
        }
        repeat(romanBuffer.length) { proxy.deleteBackward() }
        if (khmer.isNotEmpty()) proxy.insertText(khmer)
        if (suffix != null) proxy.insertText(suffix)
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

    // Mirrors Rust `is_single_keycap_char`: an ASCII graphic that is not a letter — digits,
    // punctuation, symbols. These auto-commit to a Khmer form rather than composing. Keep this in
    // sync with the Rust rule; if it widens/narrows there, update it here too.
    private fun isSingleKeycap(ch: String): Boolean =
        ch.length == 1 && ch[0].code in 0x21..0x7E && !ch[0].isLetter()
}
