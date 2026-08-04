package com.khmerime

import com.khmerime.input.TextProxy

class InMemoryTextProxy : TextProxy {

    private val buffer = StringBuilder()

    val text: String get() = buffer.toString()

    // Records the last Editor Action the handler performed (null if none yet).
    var lastEditorAction: Int? = null
        private set

    // Counts real Enter key events the handler sent (for single-line no-action fields).
    var enterKeyCount: Int = 0
        private set

    override val textBeforeCursor: String get() = buffer.toString()

    // Test-only selection model: setSelection(s) marks `s` as the trailing
    // selected region of the field; deleteSelection() removes it.
    private var selection: String? = null
    fun setSelection(text: String) { selection = text }

    override val selectedText: String? get() = selection

    override fun deleteSelection() {
        val s = selection ?: return
        val at = buffer.lastIndexOf(s)
        if (at >= 0) buffer.delete(at, at + s.length)
        selection = null
    }

    override fun insertText(text: String) {
        // Inserting over a selection replaces it (real IME behavior).
        deleteSelection()
        buffer.append(text)
    }

    override fun deleteBackward() {
        if (buffer.isNotEmpty()) buffer.deleteCharAt(buffer.length - 1)
    }

    // Test-only: simulate the host clearing the field outside the keyboard
    // (search-box ✖, select-all + delete). The keyboard is not told via a key.
    fun clearExternally() {
        buffer.setLength(0)
        selection = null
    }

    override fun performEditorAction(actionId: Int): Boolean {
        lastEditorAction = actionId
        return true
    }

    override fun sendEnterKey() {
        enterKeyCount++
    }
}
