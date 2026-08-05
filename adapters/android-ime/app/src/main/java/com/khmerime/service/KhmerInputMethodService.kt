package com.khmerime.service

import com.khmerime.input.InputConnectionProxy
import com.khmerime.input.resolveEnterBehavior
import com.khmerime.input.KhmerInputHandler
import com.khmerime.input.SmartModePreference
import com.khmerime.input.KhmerImeSession
import com.khmerime.input.KhmerRenderState
import com.khmerime.input.KeyboardState
import com.khmerime.layout.ChromeRows
import com.khmerime.layout.KeyboardKey
import com.khmerime.layout.KeyboardKeyAction
import com.khmerime.layout.KeyboardLayer
import com.khmerime.layout.KeyboardLayerSpec
import com.khmerime.layout.KeyboardPresentationSpec
import com.khmerime.layout.KeyViewFactory
import com.khmerime.layout.KeyViewStyle
import com.khmerime.layout.QwertyCharacterGridLayout
import com.khmerime.views.BackspaceKeyView
import com.khmerime.views.GlassKeyView
import com.khmerime.views.GlassKeyViewFactory
import com.khmerime.views.KeyPreviewPopup
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

    // PROCESS-WIDE singleton. The IME framework recreates this service on each
    // keyboard show/hide, and a per-instance `KhmerImeSession()` rebuilt the full
    // lexicon+stats every time (~1.5s, measured). The session is the stateless
    // engine (per-editor state lives in the handler, rebuilt each onStartInput),
    // so it is safe to build ONCE per process and reuse. (Mirrors the iOS rule:
    // "A new Session must not reload the full lexicon.")
    private var handler: KhmerInputHandler? = null
    private val mainHandler = android.os.Handler(android.os.Looper.getMainLooper())

    private var candidateStrip: LinearLayout? = null
    private var keyboardLayer: LinearLayout? = null
    private var keyPreviewPopup: KeyPreviewPopup? = null
    private var preeditStrip: PreeditStripView? = null
    private var systemBottomSpacer: View? = null
    private var candidateScroll: View? = null
    private var currentLayer = KeyboardLayer.Qwerty

    private val candidateChipPool = ViewPool<SuggestionChipView>(
        createChild = {
            SuggestionChipView(this).apply {
                // WRAP_CONTENT width: the chip sizes to its text so a whole-phrase card
                // shows the full phrase (SuggestionChipView.onMeasure); no fixed width.
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.MATCH_PARENT,
                ).apply {
                    marginStart = 4.dp()
                    marginEnd = 4.dp()
                    topMargin = 4.dp()
                    bottomMargin = 4.dp()
                }
            }
        },
        addChild = { candidateStrip?.addView(it) },
        setVisible = { view, visible -> view.visibility = if (visible) View.VISIBLE else View.GONE },
        removeChild = { candidateStrip?.removeView(it) },
    )

    // ── IME lifecycle ──────────────────────────────────────────────────────────

    override fun onStartInput(info: EditorInfo, restarting: Boolean) {
        super.onStartInput(info, restarting)
        // Wire the handler when the shared session is ready. If it's already built
        // (the common case after first launch) this runs synchronously; on the very
        // first open it defers until the background build lands — the keyboard shows
        // immediately and typing attaches a moment later. `handler == null` before
        // then makes keystrokes safe no-ops (see onKey paths).
        ensureSession({ s ->
            // still focused on the same editor?
            val ic = currentInputConnection ?: return@ensureSession
            s.setModelMode(SmartModePreference.isEnabled(this))
            val proxy = InputConnectionProxy(ic)
            handler = KhmerInputHandler(proxy, s).also { h ->
                h.enterBehavior = resolveEnterBehavior(info.imeOptions, info.inputType)
                h.onRender = ::renderState
                h.onTransition = ::renderKeyboardState
                h.onSuggestCharacterReset = ::resetSuggestCharacterSuggestions
                h.focusIn()
            }
        }, mainHandler)
    }

    // Enter behavior is resolved by the shared, unit-tested resolveEnterBehavior()
    // in the input package. See CONTEXT.md "Editor Action".

    override fun onFinishInput() {
        handler?.focusOut()
        handler = null
        super.onFinishInput()
    }

    // Fired when the cursor/selection changes — including when the host clears the
    // field externally (search-box ✖, select-all + delete). The handler resets the
    // composition + strip if our speculative roman no longer matches the field, so
    // stale suggestions don't linger after an external clear.
    override fun onUpdateSelection(
        oldSelStart: Int, oldSelEnd: Int, newSelStart: Int, newSelEnd: Int,
        candidatesStart: Int, candidatesEnd: Int,
    ) {
        super.onUpdateSelection(
            oldSelStart, oldSelEnd, newSelStart, newSelEnd, candidatesStart, candidatesEnd,
        )
        handler?.externalTextDidChange()
    }

    // ── View creation ──────────────────────────────────────────────────────────

    override fun onCreateInputView(): View {
        // The framework builds a fresh input view on a config change (rotation,
        // theme switch) while this service instance lives on. Drop the
        // service-scoped chip pool's references to the previous view's chips so
        // the old hierarchy can be garbage-collected, and so sync() re-adds
        // chips to the new candidate strip instead of leaving them on the old.
        candidateChipPool.clear()
        applyWindowBlur()
        val root = layoutInflater.inflate(R.layout.keyboard, null)
        root.setBackgroundColor(Color.TRANSPARENT)
        preeditStrip = root.findViewById<PreeditStripView>(R.id.preedit_strip).also { strip ->
            strip.onSegmentFocused = { index -> handler?.focusSegment(index) }
        }
        candidateStrip = root.findViewById(R.id.candidate_strip)
        candidateScroll = root.findViewById(R.id.candidate_scroll)
        keyboardLayer = root.findViewById(R.id.keyboard_layer)
        keyPreviewPopup = KeyPreviewPopup(this)
        systemBottomSpacer = root.findViewById(R.id.system_bottom_spacer)
        applySystemBottomSpacing(root)
        renderKeyboardLayer(KeyboardLayer.Qwerty)
        return root
    }

    override fun onFinishInputView(finishingInput: Boolean) {
        // Dismiss any live preview bubble so its PopupWindow doesn't leak when the
        // keyboard hides mid-press.
        keyPreviewPopup?.hide()
        super.onFinishInputView(finishingInput)
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
        // Seed the spacer with the REAL bottom inset before first paint so the
        // keyboard doesn't render short and then jump up when the async inset
        // listener fires a frame later. Prefer the already-attached window insets;
        // fall back to the system navigation-bar height; then a small default.
        val initialBottom = maxOf(fallbackBottom, currentBottomInset(root))
        setSystemBottomSpacerHeight(initialBottom)
        ViewCompat.setOnApplyWindowInsetsListener(root) { _, insets ->
            val bottomInset = insets
                .getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.ime())
                .bottom
            setSystemBottomSpacerHeight(maxOf(fallbackBottom, bottomInset))
            insets
        }
        ViewCompat.requestApplyInsets(root)
    }

    // Best available bottom inset at layout time, before the async listener fires.
    private fun currentBottomInset(root: View): Int {
        ViewCompat.getRootWindowInsets(root)?.let { insets ->
            val b = insets
                .getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.ime())
                .bottom
            if (b > 0) return b
        }
        // Not attached yet: use the system navigation-bar height so the first
        // frame is already the right size (the common cause of the open-jump).
        @Suppress("DiscouragedApi", "InternalInsetResource")
        val id = resources.getIdentifier("navigation_bar_height", "dimen", "android")
        return if (id > 0) resources.getDimensionPixelSize(id) else 0
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

        val rows = KeyboardLayerSpec.rows(layer)
        rows.forEachIndexed { rowIndex, keys ->
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
            addGridRow(row, keys, factory, isQwertyLetterRow(layer, rowIndex, keys))
            container.addView(row)
        }
    }

    // Rows 2 (asdfghjkl) and 3 (✦ + zxcvbnm + ⌫) of the QWERTY layer get iOS-parity
    // geometry: letter keys keep the row-1 width (weight 1); row 2 is centered with
    // side-inset spacers; row 3's edge controls widen to fill. Every other row keeps
    // the plain weighted fill.
    private fun isQwertyLetterRow(layer: KeyboardLayer, rowIndex: Int, keys: List<KeyboardKey>): Boolean =
        layer == KeyboardLayer.Qwerty && (rowIndex == 1 || rowIndex == 2)

    private fun addGridRow(
        row: LinearLayout,
        keys: List<KeyboardKey>,
        factory: KeyViewFactory,
        gridRow: Boolean,
    ) {
        // iOS QwertyCharacterGridLayout is pure ratios of the letter-key width, so a
        // representative width makes the spacer/control weights exact without a measure pass.
        val grid = QwertyCharacterGridLayout(availableWidth = REFERENCE_ROW_WIDTH, spacing = KEY_GAP_PX)
        val letterCount = keys.count { it.action == KeyboardKeyAction.Insert }

        if (gridRow && letterCount == 9) {
            // Row 2: center 9 constant-width letters with an inset spacer each side.
            val insetWeight = grid.row2SideInset / grid.characterKeyWidth
            row.addView(spacer(insetWeight))
            keys.forEach { row.addView(gridKeyView(factory, it, 1f)) }
            row.addView(spacer(insetWeight))
            return
        }
        if (gridRow && letterCount == 7) {
            // Row 3: wide edge controls around 7 constant-width letters.
            val controlWeight = grid.row3ControlWidth / grid.characterKeyWidth
            keys.forEach { key ->
                val weight = if (key.action == KeyboardKeyAction.Insert) 1f else controlWeight
                row.addView(gridKeyView(factory, key, weight))
            }
            return
        }
        // Every other row: plain weighted fill.
        keys.forEach { row.addView(gridKeyView(factory, it, KeyViewStyle.weightFor(it))) }
    }

    private fun gridKeyView(factory: KeyViewFactory, key: KeyboardKey, weight: Float): View {
        val view = factory.makeKeyView(this, key) { handleKey(key) }
        if (view is BackspaceKeyView) {
            view.onHoldFire = { handler?.backspaceHoldFired() }
            view.onHoldEnd = { handler?.backspaceHoldEnded() }
        }
        // Keypress preview bubble on letter keys only (iOS parity — no bubble over
        // space/backspace/return/toggles).
        if (key.action == KeyboardKeyAction.Insert && view is GlassKeyView) {
            view.onPreviewShow = { keyPreviewPopup?.show(it, it.previewLabel) }
            view.onPreviewHide = { keyPreviewPopup?.hide() }
        }
        view.layoutParams = LinearLayout.LayoutParams(
            0,
            LinearLayout.LayoutParams.MATCH_PARENT,
            weight,
        ).apply {
            marginStart = 2.dp()
            marginEnd = 2.dp()
        }
        return view
    }

    private fun spacer(weight: Float): View =
        View(this).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, weight)
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

    private companion object {
        // Reference dimensions for QwertyCharacterGridLayout. The layout is pure ratios
        // of the letter-key width, so any representative row width yields exact spacer /
        // edge-control weights; these approximate a typical phone row.
        const val REFERENCE_ROW_WIDTH = 360f
        const val KEY_GAP_PX = 6f

        // Process-wide session singleton, built ONCE on a BACKGROUND thread.
        // Two problems this solves:
        //   1. per-instance sessions rebuilt the lexicon (~1.5s) every keyboard
        //      open — the singleton reuses it, and makes set_model_mode's
        //      "skip if already in this mode" guard actually work.
        //   2. even the first build (~4.5s: lexicon + smart refiner) blocked the
        //      main thread. Building it off-thread lets the keyboard open
        //      instantly; onStartInput wires the handler once the session lands.
        @Volatile private var sharedSession: KhmerImeSession? = null
        private val sessionWaiters = java.util.concurrent.CopyOnWriteArrayList<(KhmerImeSession) -> Unit>()

        // Kick the build once, off the main thread. Idempotent.
        @Synchronized
        fun ensureSession(onReady: (KhmerImeSession) -> Unit, main: android.os.Handler) {
            sharedSession?.let { onReady(it); return }
            sessionWaiters.add(onReady)
            if (sessionWaiters.size > 1) return           // build already in flight
            Thread({
                val s = KhmerImeSession()
                sharedSession = s
                main.post {
                    sessionWaiters.forEach { it(s) }
                    sessionWaiters.clear()
                }
            }, "khmer-session-build").apply { isDaemon = true }.start()
        }
    }

    private fun renderKeyboardState(state: KeyboardState) {
        renderKeyboardLayer(KeyboardPresentationSpec.keyboardLayerForState(state))
        // A bare mode transition (enter/exit Suggest Character, toggle English)
        // has no composition to show; content-ful transitions are always
        // followed by a render that re-applies the real chrome.
        applyChrome(ChromeRows.None)
    }

    // Three-state input chrome (parity with iOS): collapse the rows a mode is not
    // using so the keyboard reclaims their height. See KeyboardPresentationSpec.chromeRows.
    private fun applyChrome(rows: ChromeRows) {
        preeditStrip?.visibility =
            if (rows == ChromeRows.StripAndCandidate || rows == ChromeRows.StripOnly) View.VISIBLE else View.GONE
        candidateScroll?.visibility =
            if (rows == ChromeRows.CandidateOnly || rows == ChromeRows.StripAndCandidate) View.VISIBLE else View.GONE
    }

    private fun resetSuggestCharacterSuggestions() {
        preeditStrip?.clear()
        candidateChipPool.sync(0)
        // A reset always means Suggest Character with no candidates yet → collapse.
        applyChrome(ChromeRows.None)
    }

    // ── Render ─────────────────────────────────────────────────────────────────

    private fun renderState(state: KhmerRenderState) {
        val keyboardState = handler?.keyboardState
        val romanHint = KeyboardPresentationSpec.preeditText(keyboardState, state)
        preeditStrip?.render(state, romanHint)
        applyChrome(KeyboardPresentationSpec.chromeRows(keyboardState, romanHint, state))

        if (candidateStrip == null) return
        if (KeyboardPresentationSpec.showsPhraseWheel(keyboardState, state)) {
            // Phrase Wheel: whole-phrase alternatives; a tap selects (previews in strip).
            val alternatives = KeyboardPresentationSpec.phraseAlternatives(state)
            val chips = candidateChipPool.sync(alternatives.size)
            alternatives.forEachIndexed { chipIndex, alternative ->
                chips[chipIndex].update(
                    text = KeyboardPresentationSpec.candidateDisplayLabel(alternative.text),
                    isSelected = false,
                    // ✦ marks a model-contributed phrase; red when unverified (ADR-0016), as on iOS.
                    fromModel = alternative.fromModel,
                    lexiconVerified = alternative.lexiconVerified,
                    onClick = { handler?.selectPhrase(alternative.index) },
                )
            }
        } else {
            // Word candidates (CharPick / Segment Edit): a tap selects that candidate.
            val selectedIndex = KeyboardPresentationSpec.selectedCandidateIndex(keyboardState, state)
            val candidates = KeyboardPresentationSpec.suggestionCandidates(state)
            val chips = candidateChipPool.sync(candidates.size)
            candidates.forEachIndexed { index, candidate ->
                chips[index].update(
                    text = KeyboardPresentationSpec.candidateDisplayLabel(candidate),
                    isSelected = index == selectedIndex,
                    onClick = { handler?.selectCandidate(index) },
                )
            }
        }
    }
}
