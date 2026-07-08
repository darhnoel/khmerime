package com.khmerime.input

import com.google.gson.Gson

// KhmerImeSession
// ===============
// JNI wrapper around the Rust ImeSession. The Rust object is heap-allocated;
// its address is stored in `nativeHandle` and must be freed via `nativeDestroy`
// when no longer needed.
//
// Every method serialises the Rust `RenderState` to JSON and deserialises here
// so that the Kotlin side never manipulates Rust memory directly.

class KhmerImeSession {

    private val nativeHandle: Long = nativeCreate()

    fun focusIn(): KhmerRenderState = parse(nativeFocusIn(nativeHandle))
    fun focusOut(): KhmerRenderState = parse(nativeFocusOut(nativeHandle))
    fun processCharacter(ch: String): KhmerRenderState = parse(nativeProcessCharacter(nativeHandle, ch))
    fun processBackspace(): KhmerRenderState = parse(nativeProcessBackspace(nativeHandle))
    fun processSpace(): KhmerRenderState = parse(nativeProcessSpace(nativeHandle))
    fun processEnter(): KhmerRenderState = parse(nativeProcessEnter(nativeHandle))
    fun processLeft(): KhmerRenderState = parse(nativeProcessLeft(nativeHandle))
    fun processRight(): KhmerRenderState = parse(nativeProcessRight(nativeHandle))
    fun processTab(): KhmerRenderState = parse(nativeProcessTab(nativeHandle))
    fun processDigit(n: Int): KhmerRenderState = parse(nativeProcessDigit(nativeHandle, n))
    fun selectPhrase(index: Int): KhmerRenderState = parse(nativeSelectPhrase(nativeHandle, index))
    fun enterCharPick(): KhmerRenderState = parse(nativeEnterCharPick(nativeHandle))
    fun exitCharPick(): KhmerRenderState = parse(nativeExitCharPick(nativeHandle))

    private external fun nativeCreate(): Long
    private external fun nativeDestroy(handle: Long)
    private external fun nativeFocusIn(handle: Long): String
    private external fun nativeFocusOut(handle: Long): String
    private external fun nativeProcessCharacter(handle: Long, ch: String): String
    private external fun nativeProcessBackspace(handle: Long): String
    private external fun nativeProcessSpace(handle: Long): String
    private external fun nativeProcessEnter(handle: Long): String
    private external fun nativeProcessLeft(handle: Long): String
    private external fun nativeProcessRight(handle: Long): String
    private external fun nativeProcessTab(handle: Long): String
    private external fun nativeProcessDigit(handle: Long, n: Int): String
    private external fun nativeSelectPhrase(handle: Long, index: Int): String
    private external fun nativeEnterCharPick(handle: Long): String
    private external fun nativeExitCharPick(handle: Long): String

    companion object {
        init {
            System.loadLibrary("khmerime_android_ime")
        }

        private val gson = Gson()

        private fun parse(json: String): KhmerRenderState =
            gson.fromJson(json, KhmerRenderState::class.java)
    }
}
