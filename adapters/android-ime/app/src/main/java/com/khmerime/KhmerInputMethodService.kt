package com.khmerime

import android.inputmethodservice.InputMethodService
import android.view.ContextThemeWrapper
import android.view.Gravity
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

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
    private var keyboardLayer: LinearLayout? = null
    private var preeditBar: TextView? = null
    private var systemBottomSpacer: View? = null
    private var currentLayer = KeyboardLayer.Qwerty

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
        keyboardLayer = root.findViewById(R.id.keyboard_layer)
        systemBottomSpacer = root.findViewById(R.id.system_bottom_spacer)
        applySystemBottomSpacing(root)
        renderKeyboardLayer(KeyboardLayer.Qwerty)
        return root
    }

    private fun applySystemBottomSpacing(root: View) {
        val fallbackBottom = 12.dp()
        setSystemBottomSpacerHeight(fallbackBottom)
        ViewCompat.setOnApplyWindowInsetsListener(root) { _, insets ->
            val bottomInset = insets
                .getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.ime())
                .bottom
            setSystemBottomSpacerHeight(maxOf(fallbackBottom, bottomInset))
            insets
        }
        ViewCompat.requestApplyInsets(root)
    }

    private fun setSystemBottomSpacerHeight(height: Int) {
        val spacer = systemBottomSpacer ?: return
        val params = spacer.layoutParams
        if (params.height != height) {
            params.height = height
            spacer.layoutParams = params
        }
    }

    // ── Key wiring ─────────────────────────────────────────────────────────────

    private fun renderKeyboardLayer(layer: KeyboardLayer) {
        currentLayer = layer
        val container = keyboardLayer ?: return
        container.removeAllViews()

        KeyboardLayerSpec.rows(layer).forEach { keys ->
            val row = LinearLayout(this).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    0,
                    1f,
                ).apply {
                    topMargin = 2.dp()
                    bottomMargin = 2.dp()
                }
            }
            keys.forEach { key ->
                row.addView(makeKeyButton(key))
            }
            container.addView(row)
        }
    }

    private fun makeKeyButton(key: KeyboardKey): Button {
        val style = if (key.action == KeyboardKeyAction.Insert) {
            R.style.KeyButton
        } else {
            R.style.ActionButton
        }
        return Button(ContextThemeWrapper(this, style)).apply {
            text = key.label
            isAllCaps = false
            textSize = if (key.label.length > 1) 13f else 16f
            setOnClickListener { handleKey(key) }
            layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.MATCH_PARENT,
                keyWeight(key),
            ).apply {
                marginStart = 2.dp()
                marginEnd = 2.dp()
            }
        }
    }

    private fun keyWeight(key: KeyboardKey): Float = when (key.action) {
        KeyboardKeyAction.Space -> 4f
        KeyboardKeyAction.Return -> 1.4f
        KeyboardKeyAction.TogglePanel,
        KeyboardKeyAction.Backspace,
        KeyboardKeyAction.NextKeyboard,
        KeyboardKeyAction.SwitchToQwerty,
        KeyboardKeyAction.SwitchToNumeric,
        KeyboardKeyAction.SwitchToSymbols -> 1.3f
        KeyboardKeyAction.Insert -> 1f
    }

    private fun handleKey(key: KeyboardKey) {
        when (key.action) {
            KeyboardKeyAction.Insert -> handler?.sendChar(key.input)
            KeyboardKeyAction.Backspace -> handler?.sendBackspace()
            KeyboardKeyAction.Space -> handler?.sendSpace()
            KeyboardKeyAction.Return -> handler?.sendReturn()
            KeyboardKeyAction.SwitchToQwerty -> renderKeyboardLayer(KeyboardLayer.Qwerty)
            KeyboardKeyAction.SwitchToNumeric -> renderKeyboardLayer(KeyboardLayer.Numeric)
            KeyboardKeyAction.SwitchToSymbols -> renderKeyboardLayer(KeyboardLayer.Symbols)
            KeyboardKeyAction.TogglePanel -> Unit
            KeyboardKeyAction.NextKeyboard -> Unit
        }
    }

    private fun Int.dp(): Int = (this * resources.displayMetrics.density).toInt()

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
