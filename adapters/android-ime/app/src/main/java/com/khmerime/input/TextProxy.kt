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

    // Performs the host field's Editor Action (Search / Go / Send / …) instead of
    // inserting text. Returns whether the field consumed it. See CONTEXT.md
    // "Editor Action".
    fun performEditorAction(actionId: Int): Boolean

    // Sends a real KEYCODE_ENTER key event (down + up). Used for single-line
    // fields with no declared Editor Action, which submit on the Enter key
    // rather than on a committed newline. See CONTEXT.md "Editor Action".
    fun sendEnterKey()
}
