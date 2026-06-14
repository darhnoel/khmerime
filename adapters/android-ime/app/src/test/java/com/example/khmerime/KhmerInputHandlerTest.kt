package com.example.khmerime

import org.junit.Assert.*
import org.junit.Test

// KhmerInputHandlerTest
// =====================
// Integration tests: real Rust session via JNI, MockTextProxy.
// Mirrors KeyboardInputHandlerTests on iOS — behavior only, never internals.

class KhmerInputHandlerTest {

    private fun makeHandler(): Pair<KhmerInputHandler, MockTextProxy> {
        val proxy = MockTextProxy()
        val session = KhmerImeSession()
        val handler = KhmerInputHandler(proxy, session)
        handler.focusIn()
        return Pair(handler, proxy)
    }

    private fun type(word: String, into: KhmerInputHandler) {
        for (ch in word) into.sendChar(ch.toString())
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    @Test
    fun focusInThenFocusOut_leavesProxyEmpty() {
        val (handler, proxy) = makeHandler()

        handler.focusOut()

        assertTrue("focusIn + focusOut must leave proxy empty", proxy.text.isEmpty())
    }

    // ── Tracer bullet ──────────────────────────────────────────────────────────

    @Test
    fun typing_speculativelyInsertsRomanIntoProxy() {
        val (handler, proxy) = makeHandler()

        type("nh", into = handler)

        assertEquals("proxy must reflect roman preedit speculatively", "nh", proxy.text)
    }

    @Test
    fun sendReturn_withEmptyBuffer_insertsNewline() {
        val (handler, proxy) = makeHandler()

        handler.sendReturn()

        assertEquals("empty preedit + Return must insert newline", "\n", proxy.text)
    }

    @Test
    fun sendBackspace_removesLastCharFromProxy() {
        val (handler, proxy) = makeHandler()
        type("nh", into = handler)

        handler.sendBackspace()

        assertEquals("backspace must shorten proxy by one char", "n", proxy.text)
    }

    @Test
    fun typeAndEnter_commitsKhmerToProxy() {
        val (handler, proxy) = makeHandler()
        type("nhom", into = handler)

        handler.sendReturn()

        assertFalse("proxy must have committed text", proxy.text.isEmpty())
        assertFalse("committed text must not be roman", proxy.text.contains("nhom"))
        val isKhmer = proxy.text.all { it.code in 0x1780..0x17FF }
        assertTrue(
            "committed text must be Khmer Unicode; got '${proxy.text}'",
            isKhmer,
        )
    }
}
