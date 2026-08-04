package com.khmerime.input

// TextProxy
// =========
// Abstracts text-field operations so KhmerInputHandler can be unit-tested
// without a real InputConnection. Mirrors iOS TextProxy.

interface TextProxy {
    fun insertText(text: String)
    fun deleteBackward()

    // Delete `count` chars before the cursor in a single operation. Replacing a
    // roman buffer with committed Khmer would otherwise fire one IPC per char;
    // batching is one cross-process call. Default keeps existing callers working.
    fun deleteBackward(count: Int) {
        repeat(count) { deleteBackward() }
    }

    val textBeforeCursor: String?

    // Non-null when the host field has a non-empty selection. Backspace (and any
    // char insert) must replace the whole selection, not delete a single char —
    // deleteSurroundingText(1,0) does not touch a selection. Default null keeps
    // proxies that don't model selection (tests, simple fields) behaving as before.
    val selectedText: String? get() = null

    // Replace the current selection with nothing (bulk-delete selected text).
    // Default no-op for proxies without selection support.
    fun deleteSelection() {}

    // Performs the host field's Editor Action (Search / Go / Send / …) instead of
    // inserting text. Returns whether the field consumed it. See CONTEXT.md
    // "Editor Action".
    fun performEditorAction(actionId: Int): Boolean

    // Sends a real KEYCODE_ENTER key event (down + up). Used for single-line
    // fields with no declared Editor Action, which submit on the Enter key
    // rather than on a committed newline. See CONTEXT.md "Editor Action".
    fun sendEnterKey()
}
