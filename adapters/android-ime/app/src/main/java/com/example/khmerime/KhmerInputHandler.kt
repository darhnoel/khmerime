package com.example.khmerime

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

    var onRender: ((KhmerRenderState) -> Unit)? = null

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    fun focusIn() {
        render(session.focusIn())
    }

    fun focusOut() {
        session.focusOut()
    }

    // ── Key actions ───────────────────────────────────────────────────────────

    fun sendChar(ch: String) {
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
        if (romanBuffer.isNotEmpty()) romanBuffer = romanBuffer.dropLast(1)
        proxy.deleteBackward()
        render(session.processBackspace())
    }

    fun sendSpace() {
        commitComposition()
        proxy.insertText(" ")
    }

    fun sendReturn() {
        if (romanBuffer.isNotEmpty()) {
            commitComposition()
            return
        }
        proxy.insertText("\n")
    }

    // ── Private ───────────────────────────────────────────────────────────────

    private fun commitComposition() {
        val state = session.processEnter()
        val khmer = state.commitText ?: ""
        repeat(romanBuffer.length) { proxy.deleteBackward() }
        if (khmer.isNotEmpty()) proxy.insertText(khmer)
        romanBuffer = ""
        render(state)
    }

    private fun render(state: KhmerRenderState) {
        onRender?.invoke(state)
    }
}
