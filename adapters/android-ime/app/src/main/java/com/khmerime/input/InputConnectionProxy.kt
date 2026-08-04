package com.khmerime.input

import android.view.KeyEvent
import android.view.inputmethod.InputConnection

// InputConnectionProxy
// ====================
// Wraps a live InputConnection so KhmerInputHandler can insert and delete text
// without knowing about the Android framework. Implements the same TextProxy
// interface used by InMemoryTextProxy in tests.

class InputConnectionProxy(private val ic: InputConnection) : TextProxy {

    override fun insertText(text: String) {
        ic.commitText(text, 1)
    }

    override fun deleteBackward() {
        ic.deleteSurroundingText(1, 0)
    }

    // One IPC deletes the whole roman buffer instead of one call per char.
    override fun deleteBackward(count: Int) {
        if (count > 0) ic.deleteSurroundingText(count, 0)
    }

    override val textBeforeCursor: String?
        get() = ic.getTextBeforeCursor(256, 0)?.toString()

    // Non-null only when a non-empty selection exists. getSelectedText returns the
    // selected span (or null/empty when the cursor is collapsed).
    override val selectedText: String?
        get() = ic.getSelectedText(0)?.toString()?.takeIf { it.isNotEmpty() }

    // Replace the selection with nothing. commitText("") over a selection deletes
    // it in one IPC — deleteSurroundingText(1,0) would not touch the selection.
    override fun deleteSelection() {
        ic.commitText("", 1)
    }

    override fun performEditorAction(actionId: Int): Boolean = ic.performEditorAction(actionId)

    override fun sendEnterKey() {
        ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ENTER))
        ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_ENTER))
    }
}
