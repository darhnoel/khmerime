package com.khmerime.views

import com.khmerime.layout.KeyboardKey
import android.content.Context
import android.view.MotionEvent

// BackspaceKeyView
// ================
// GlassKeyView with long-press repeat — the standard keyboard feel where
// holding ⌫ fires repeated deletes. Repeat/batching logic lives in
// BackspaceRepeater (no Android dependency); this view only translates
// touch events into tap/hold callbacks. Mirrors iOS's BackspaceButton.
//
// Wiring (in KhmerInputMethodService):
//   view.onTap      = { handler?.sendBackspace() }
//   view.onHoldFire = { handler?.backspaceHoldFired() }
//   view.onHoldEnd  = { handler?.backspaceHoldEnded() }

class BackspaceKeyView(
    context: Context,
    key: KeyboardKey,
    isActive: Boolean = false,
    private val repeater: BackspaceRepeater = BackspaceRepeater(),
) : GlassKeyView(context, key, isActive, onClick = {}) {

    var onTap: (() -> Unit)? = null
    var onHoldFire: (() -> Unit)? = null
    var onHoldEnd: (() -> Unit)? = null

    private val pressAnimator = GlassKeyPressAnimator(onUpdate = { applySquish(it) })

    init {
        repeater.onFire = { onHoldFire?.invoke() }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                capturePressedBounds(event)
                performKeyPressHaptic()
                pressAnimator.press()
                repeater.beginHold()
                return true
            }
            MotionEvent.ACTION_UP -> {
                pressAnimator.release()
                if (repeater.hasFired) {
                    onHoldEnd?.invoke()
                } else if (releaseIsInsidePressedBounds(event, slop = 0f)) {
                    onTap?.invoke()
                }
                clearPressedBounds()
                repeater.endHold()
                return true
            }
            MotionEvent.ACTION_CANCEL -> {
                clearPressedBounds()
                pressAnimator.release()
                if (repeater.hasFired) onHoldEnd?.invoke()
                repeater.endHold()
                return true
            }
        }
        return super.onTouchEvent(event)
    }
}
