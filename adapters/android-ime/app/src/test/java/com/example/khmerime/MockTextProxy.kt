package com.example.khmerime

class MockTextProxy : TextProxy {

    private val buffer = StringBuilder()

    val text: String get() = buffer.toString()

    override val textBeforeCursor: String get() = buffer.toString()

    override fun insertText(text: String) {
        buffer.append(text)
    }

    override fun deleteBackward() {
        if (buffer.isNotEmpty()) buffer.deleteCharAt(buffer.length - 1)
    }
}
