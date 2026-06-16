package com.khmerime.service

import com.khmerime.input.InputConnectionProxy
import com.khmerime.input.KhmerInputHandler
import com.khmerime.input.KhmerImeSession
import com.khmerime.input.KhmerRenderState
import com.khmerime.input.KeyboardState
import com.khmerime.layout.KeyboardKey
import com.khmerime.layout.KeyboardKeyAction
import com.khmerime.layout.KeyboardLayer
import com.khmerime.layout.KeyboardLayerSpec
import com.khmerime.layout.KeyboardPresentationSpec
import com.khmerime.layout.KeyViewFactory
import com.khmerime.layout.KeyViewStyle
import com.khmerime.views.BackspaceKeyView
import com.khmerime.views.GlassKeyViewFactory
import com.khmerime.views.PreeditStripView
import com.khmerime.views.SuggestionChipView
import com.khmerime.views.ViewPool
import com.khmerime.R
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.inputmethodservice.InputMethodService
import android.os.Build
import android.view.Gravity
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.LinearLayout
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
    private var preeditStrip: PreeditStripView? = null
    private var systemBottomSpacer: View? = null
    private var currentLayer = KeyboardLayer.Qwerty

    private val candidateChipPool = ViewPool<SuggestionChipView>(
        createChild = {
            SuggestionChipView(this).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.MATCH_PARENT,
                ).apply {
                    marginStart = 4.dp()
                    marginEnd = 4.dp()
                    topMargin = 4.dp()
                    bottomMargin = 4.dp()
                    width = 80.dp()
                }
            }
        },
        addChild = { candidateStrip?.addView(it) },
        setVisible = { view, visible -> view.visibility = if (visible) View.VISIBLE else View.GONE },
    )

    // ── IME lifecycle ──────────────────────────────────────────────────────────

    override fun onStartInput(info: EditorInfo, restarting: Boolean) {
        super.onStartInput(info, restarting)
        val ic = currentInputConnection ?: return
        val proxy = InputConnectionProxy(ic)
        handler = KhmerInputHandler(proxy, session).also { h ->
            h.onRender = ::renderState
            h.onTransition = ::renderKeyboardState
            h.onSuggestCharacterReset = ::resetSuggestCharacterSuggestions
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
        applyWindowBlur()
        val root = layoutInflater.inflate(R.layout.keyboard, null)
        root.setBackgroundColor(Color.TRANSPARENT)
        preeditStrip = root.findViewById<PreeditStripView>(R.id.preedit_strip).also { strip ->
            strip.onSegmentFocused = { index -> handler?.focusSegment(index) }
        }
        candidateStrip = root.findViewById(R.id.candidate_strip)
        keyboardLayer = root.findViewById(R.id.keyboard_layer)
        systemBottomSpacer = root.findViewById(R.id.system_bottom_spacer)
        applySystemBottomSpacing(root)
        renderKeyboardLayer(KeyboardLayer.Qwerty)
        return root
    }

    private fun applyWindowBlur() {
        val win = window?.window ?: return
        win.setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
        if (Build.VERSION.SDK_INT >= 31) {
            val radiusPx = (20 * resources.displayMetrics.density).toInt()
            win.setBackgroundBlurRadius(radiusPx)
        }
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

        val factory: KeyViewFactory = GlassKeyViewFactory(
            handler?.keyboardState ?: KeyboardState.Qwerty,
        )

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
                val view = factory.makeKeyView(this, key) { handleKey(key) }
                if (view is BackspaceKeyView) {
                    view.onHoldFire = { handler?.backspaceHoldFired() }
                    view.onHoldEnd = { handler?.backspaceHoldEnded() }
                }
                view.layoutParams = LinearLayout.LayoutParams(
                    0,
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    KeyViewStyle.weightFor(key),
                ).apply {
                    marginStart = 2.dp()
                    marginEnd = 2.dp()
                }
                row.addView(view)
            }
            container.addView(row)
        }
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
            KeyboardKeyAction.TogglePanel -> handler?.toggleSuggestCharacter()
            KeyboardKeyAction.ToggleEnglish -> handler?.toggleEnglish()
            KeyboardKeyAction.NextKeyboard -> Unit
        }
    }

    private fun Int.dp(): Int = (this * resources.displayMetrics.density).toInt()

    private fun renderKeyboardState(state: KeyboardState) {
        renderKeyboardLayer(KeyboardPresentationSpec.keyboardLayerForState(state))
    }

    private fun resetSuggestCharacterSuggestions() {
        preeditStrip?.clear()
        candidateChipPool.sync(0)
    }

    // ── Render ─────────────────────────────────────────────────────────────────

    private fun renderState(state: KhmerRenderState) {
        val keyboardState = handler?.keyboardState
        val romanHint = KeyboardPresentationSpec.preeditText(keyboardState, state)
        preeditStrip?.render(state, romanHint)

        if (candidateStrip == null) return
        val selectedIndex = KeyboardPresentationSpec.selectedCandidateIndex(state)
        val candidates = KeyboardPresentationSpec.suggestionCandidates(state)
        val chips = candidateChipPool.sync(candidates.size)
        candidates.forEachIndexed { index, candidate ->
            chips[index].update(
                text = candidate,
                isSelected = index == selectedIndex,
                onClick = { handler?.selectCandidate(index) },
            )
        }
    }
}
