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
) {

    private var romanBuffer = ""
    private var trailingSpace = false
    private var lastState: KhmerRenderState? = null

    var keyboardState: KeyboardState = KeyboardState.Qwerty
        private set

    var onRender: ((KhmerRenderState) -> Unit)? = null
    var onTransition: ((KeyboardState) -> Unit)? = null
    var onSuggestCharacterReset: (() -> Unit)? = null

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    fun focusIn() {
        render(session.focusIn())
    }

    fun focusOut() {
        session.focusOut()
    }

    // ── Key actions ───────────────────────────────────────────────────────────

    fun sendChar(ch: String) {
        if (keyboardState == KeyboardState.English) {
            proxy.insertText(ch)
            return
        }
        if (keyboardState == KeyboardState.SuggestCharacter) {
            render(session.processCharacter(ch))
            return
        }
        if (keyboardState == KeyboardState.Panel) transitionTo(KeyboardState.Qwerty)
        trailingSpace = false
        proxy.insertText(ch)
        romanBuffer += ch
        val state = session.processCharacter(ch)
        val committed = state.commitText
        if (committed != null && committed.isNotEmpty()) {
            repeat(romanBuffer.length) { proxy.deleteBackward() }
            proxy.insertText(committed)
            romanBuffer = ""
        }
        render(state)
    }

    fun sendBackspace() {
        if (keyboardState == KeyboardState.English) {
            proxy.deleteBackward()
            return
        }
        trailingSpace = false
        if (keyboardState == KeyboardState.SuggestCharacter) {
            val current = lastState
            if (current != null && current.candidates.isNotEmpty()) {
                session.enterCharPick()
                lastState = null
                onSuggestCharacterReset?.invoke()
                transitionTo(KeyboardState.SuggestCharacter)
            } else {
                proxy.deleteBackward()
            }
            return
        }
        if (romanBuffer.isNotEmpty()) romanBuffer = romanBuffer.dropLast(1)
        proxy.deleteBackward()
        render(session.processBackspace())
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
            proxy.insertText("\n")
            return
        }
        if (keyboardState == KeyboardState.SuggestCharacter) {
            selectCandidate(0)
            return
        }
        if (romanBuffer.isNotEmpty()) {
            commitComposition()
            return
        }
        if (trailingSpace) {
            proxy.deleteBackward()
            trailingSpace = false
        }
        proxy.insertText("\n")
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

    // ── Private ───────────────────────────────────────────────────────────────

    private fun commitComposition() {
        if (keyboardState == KeyboardState.SuggestCharacter) return
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
