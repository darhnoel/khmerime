package com.example.khmerime

import android.inputmethodservice.InputMethodService
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

// KhmerInputMethodService
// =======================
// Android InputMethodService that wires KhmerInputHandler to the live
// InputConnection. The session is created once (service lifetime); the handler
// is re-created on each onStartInput so the proxy always points at the fresh
// InputConnection for the focused editor.
//
// To install on a real device the Rust crate must be cross-compiled for the
// device ABI. Use cargo-ndk:
//   cargo install cargo-ndk
//   rustup target add aarch64-linux-android
//   cargo ndk -t arm64-v8a -o app/src/main/jniLibs build
// then rebuild the APK.

class KhmerInputMethodService : InputMethodService() {

    private val session = KhmerImeSession()
    private var handler: KhmerInputHandler? = null

    private var candidateStrip: LinearLayout? = null
    private var preeditBar: TextView? = null

    // ── IME lifecycle ──────────────────────────────────────────────────────────

    override fun onStartInput(info: EditorInfo, restarting: Boolean) {
        super.onStartInput(info, restarting)
        val ic = currentInputConnection ?: return
        val proxy = InputConnectionProxy(ic)
        handler = KhmerInputHandler(proxy, session).also { h ->
            h.onRender = ::renderState
            h.focusIn()
        }
    }

    override fun onFinishInput() {
        handler?.focusOut()
        handler = null
        super.onFinishInput()
    }

    // ── View creation ──────────────────────────────────────────────────────────

    override fun onCreateInputView(): View {
        val root = layoutInflater.inflate(R.layout.keyboard, null)
        preeditBar = root.findViewById(R.id.preedit_bar)
        candidateStrip = root.findViewById(R.id.candidate_strip)
        wireKeys(root)
        return root
    }

    // ── Key wiring ─────────────────────────────────────────────────────────────

    private val LETTERS = "qwertyuiopasdfghjklzxcvbnm"

    private fun wireKeys(root: View) {
        for (ch in LETTERS) {
            root.findViewWithTag<Button>(ch.toString())?.setOnClickListener {
                handler?.sendChar(ch.toString())
            }
        }
        root.findViewById<Button>(R.id.key_backspace).setOnClickListener {
            handler?.sendBackspace()
        }
        root.findViewById<Button>(R.id.key_space).setOnClickListener {
            handler?.sendSpace()
        }
        root.findViewById<Button>(R.id.key_enter).setOnClickListener {
            handler?.sendReturn()
        }
    }

    // ── Render ─────────────────────────────────────────────────────────────────

    private fun renderState(state: KhmerRenderState) {
        preeditBar?.text = state.preedit

        val strip = candidateStrip ?: return
        strip.removeAllViews()
        state.candidates.forEachIndexed { index, candidate ->
            val btn = Button(this).apply {
                text = candidate
                setOnClickListener {
                    // Select candidate: delete roman buffer, insert Khmer candidate
                    currentInputConnection?.let { ic ->
                        val proxy = InputConnectionProxy(ic)
                        repeat(state.preedit.length) { proxy.deleteBackward() }
                        proxy.insertText(candidate)
                    }
                    this@KhmerInputMethodService.handler?.focusIn()
                }
                textSize = 16f
            }
            strip.addView(btn)
        }
    }
}
