package com.example.khmerime

// TextProxy
// =========
// Abstracts text-field operations so KhmerInputHandler can be unit-tested
// without a real InputConnection. Mirrors iOS TextProxy.

interface TextProxy {
    fun insertText(text: String)
    fun deleteBackward()
    val textBeforeCursor: String?
}
