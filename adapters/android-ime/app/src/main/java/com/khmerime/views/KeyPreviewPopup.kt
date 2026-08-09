package com.khmerime.views

import android.content.Context
import android.graphics.Color
import android.graphics.Rect
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.widget.PopupWindow
import android.widget.TextView
import kotlin.math.roundToInt

// A non-touchable screen-space popup. Absolute placement avoids PopupWindow's
// anchor-relative auto-flip and lets top-row previews draw above the IME window.
class KeyPreviewPopup(context: Context) {

    private val density = context.resources.displayMetrics.density
    private val screenWidth = context.resources.displayMetrics.widthPixels.toFloat()
    private val anchorLocation = IntArray(2)
    private val rootLocation = IntArray(2)
    private val sourceRect = Rect()
    private val label = TextView(context).apply {
        gravity = Gravity.CENTER
        textSize = 26f
        setTextColor(if (isDark(context)) Color.WHITE else 0xFF111111.toInt())
        background = GradientDrawable().apply {
            cornerRadius = 10f * density
            setColor(if (isDark(context)) 0xFF3A3A3C.toInt() else Color.WHITE)
            setStroke(
                density.roundToInt().coerceAtLeast(1),
                if (isDark(context)) 0x33FFFFFF else 0x22000000,
            )
        }
    }
    private val window = PopupWindow(label, 0, 0, false).apply {
        isTouchable = false
        isOutsideTouchable = false
        isClippingEnabled = false
        setIsLaidOutInScreen(true)
        inputMethodMode = PopupWindow.INPUT_METHOD_NOT_NEEDED
        elevation = 6f * density
    }

    fun show(anchor: GlassKeyView, text: String) {
        val root = anchor.rootView
        if (anchor.width <= 0 || anchor.height <= 0 || root.width <= 0) return

        anchor.copyVisualBounds(sourceRect)
        anchor.getLocationOnScreen(anchorLocation)
        root.getLocationOnScreen(rootLocation)
        val sourceInIme = PopupRect(
            left = anchorLocation[0] - rootLocation[0] + sourceRect.left.toFloat(),
            top = anchorLocation[1] - rootLocation[1] + sourceRect.top.toFloat(),
            right = anchorLocation[0] - rootLocation[0] + sourceRect.right.toFloat(),
            bottom = anchorLocation[1] - rootLocation[1] + sourceRect.bottom.toFloat(),
        )
        val frame = absoluteKeyPreviewPopupFrame(
            sourceInIme = sourceInIme,
            imeLeftOnScreen = rootLocation[0].toFloat(),
            imeTopOnScreen = rootLocation[1].toFloat(),
            screenWidth = screenWidth,
        )

        label.animate().cancel()
        label.alpha = 1f
        label.text = text
        val x = frame.left.roundToInt()
        val y = frame.top.roundToInt()
        val width = frame.width.roundToInt()
        val height = frame.height.roundToInt()
        if (window.isShowing) {
            window.update(x, y, width, height)
        } else {
            window.width = width
            window.height = height
            window.showAtLocation(root, Gravity.TOP or Gravity.START, x, y)
        }
    }

    fun hide() {
        if (!window.isShowing) return
        label.animate().cancel()
        label.animate().alpha(0f).setDuration(HIDE_FADE_MS).withEndAction {
            if (label.alpha == 0f) window.dismiss()
        }.start()
    }

    fun hideImmediately() {
        label.animate().cancel()
        window.dismiss()
    }

    private fun isDark(context: Context): Boolean =
        (context.resources.configuration.uiMode and
            android.content.res.Configuration.UI_MODE_NIGHT_MASK) ==
            android.content.res.Configuration.UI_MODE_NIGHT_YES

    private companion object {
        const val HIDE_FADE_MS = 45L
    }
}
