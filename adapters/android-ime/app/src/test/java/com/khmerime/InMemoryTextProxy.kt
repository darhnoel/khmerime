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

    override fun insertText(text: String) {
        buffer.append(text)
    }

    override fun deleteBackward() {
        if (buffer.isNotEmpty()) buffer.deleteCharAt(buffer.length - 1)
    }

    override fun performEditorAction(actionId: Int): Boolean {
        lastEditorAction = actionId
        return true
    }

    override fun sendEnterKey() {
        enterKeyCount++
    }
}
