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

    override val textBeforeCursor: String?
        get() = ic.getTextBeforeCursor(256, 0)?.toString()

    override fun performEditorAction(actionId: Int): Boolean = ic.performEditorAction(actionId)

    override fun sendEnterKey() {
        ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ENTER))
        ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_ENTER))
    }
}
