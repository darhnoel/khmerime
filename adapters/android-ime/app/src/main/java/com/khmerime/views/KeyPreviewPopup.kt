package com.khmerime.views

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.widget.PopupWindow
import android.widget.TextView

// Keypress preview bubble (iOS KeyPreviewPopupView parity): a rounded, magnified
// label floating above the pressed letter key. Backed by a PopupWindow so it draws
// above the keyboard without restructuring the input view. Sizing/positioning comes
// from the unit-tested keyPreviewPopupFrame().
class KeyPreviewPopup(context: Context) {

    private val density = context.resources.displayMetrics.density

    private val dark = isDark(context)

    private val label = TextView(context).apply {
        gravity = Gravity.CENTER
        setTextColor(if (dark) Color.WHITE else 0xFF111111.toInt())
        textSize = 26f
        // Built once, not per keystroke — rebuilding a GradientDrawable on every
        // ACTION_DOWN was needless allocation on the hot path.
        background = bubbleBackground(dark)
    }

    private val location = IntArray(2)

    private val window = PopupWindow(label, 0, 0, false).apply {
        isClippingEnabled = false // allow the bubble to sit above the keyboard top edge
        isTouchable = false
    }

    // Show above `anchor` (a GlassKeyView). The frame math mirrors iOS; PopupWindow
    // offsets are relative to the anchor's own top-left, so convert from absolute.
    fun show(anchor: GlassKeyView, text: String) {
        label.text = text

        val loc = location.also { anchor.getLocationInWindow(it) }
        val root = anchor.rootView
        val source = PopupRect(
            left = loc[0].toFloat(),
            top = loc[1].toFloat(),
            right = (loc[0] + anchor.width).toFloat(),
            bottom = (loc[1] + anchor.height).toFloat(),
        )
        val bounds = PopupRect(0f, 0f, root.width.toFloat(), root.height.toFloat())
        val frame = keyPreviewPopupFrame(source, bounds)

        window.width = frame.width.toInt()
        window.height = frame.height.toInt()
        val offsetX = (frame.left - loc[0]).toInt()
        val offsetY = (frame.top - loc[1]).toInt()
        if (window.isShowing) {
            window.update(anchor, offsetX, offsetY, frame.width.toInt(), frame.height.toInt())
        } else {
            window.showAsDropDown(anchor, offsetX, offsetY)
        }
    }

    fun hide() {
        if (window.isShowing) window.dismiss()
    }

    private fun bubbleBackground(dark: Boolean): GradientDrawable =
        GradientDrawable().apply {
            cornerRadius = 10f * density
            setColor(if (dark) 0xFF3A3A3C.toInt() else 0xFFFFFFFF.toInt())
            setStroke((1f * density).toInt(), if (dark) 0x33FFFFFF else 0x22000000)
        }

    private fun isDark(context: Context): Boolean =
        (context.resources.configuration.uiMode and
            android.content.res.Configuration.UI_MODE_NIGHT_MASK) ==
            android.content.res.Configuration.UI_MODE_NIGHT_YES
}
