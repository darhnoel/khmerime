package com.khmerime.views

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.RectF
import android.util.AttributeSet
import android.util.TypedValue
import android.view.View
import android.view.ViewGroup

// A key preview drawn inside the IME's own root view. Unlike PopupWindow, both
// the key and bubble use keyboard-local coordinates, so host-app insets cannot
// shift or flip the preview.
class KeyPreviewOverlay @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
) : View(context, attrs) {

    var previewFrame: PopupRect? = null
        private set

    private var label: String = ""
    private val sourceRect = Rect()
    private val drawRect = RectF()
    private val density = resources.displayMetrics.density
    private val backgroundPaint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val borderPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = density
    }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        textAlign = Paint.Align.CENTER
        textSize = TypedValue.applyDimension(
            TypedValue.COMPLEX_UNIT_SP,
            26f,
            resources.displayMetrics,
        )
    }

    fun show(anchor: View, text: String) {
        val root = parent as? ViewGroup ?: return
        if (root.width <= 0 || root.height <= 0) {
            previewFrame = null
            return
        }
        layoutParams = layoutParams.apply {
            width = root.width
            height = root.height
        }
        visibility = VISIBLE
        layout(0, 0, root.width, root.height)
        sourceRect.set(0, 0, anchor.width, anchor.height)
        root.offsetDescendantRectToMyCoords(anchor, sourceRect)

        val source = PopupRect(
            sourceRect.left.toFloat(),
            sourceRect.top.toFloat(),
            sourceRect.right.toFloat(),
            sourceRect.bottom.toFloat(),
        )
        previewFrame = keyPreviewPopupFrame(
            source,
            PopupRect(0f, 0f, width.toFloat(), height.toFloat()),
        )
        label = text
        animate().cancel()
        alpha = 1f
        invalidate()
    }

    fun hide() {
        if (visibility != VISIBLE) return
        animate().cancel()
        animate().alpha(0f).setDuration(HIDE_FADE_MS).withEndAction {
            if (alpha == 0f) visibility = GONE
        }.start()
    }

    fun hideImmediately() {
        animate().cancel()
        alpha = 0f
        visibility = GONE
        previewFrame = null
    }

    override fun onDraw(canvas: Canvas) {
        val frame = previewFrame ?: return
        val dark = isDark()
        backgroundPaint.color = if (dark) 0xFF3A3A3C.toInt() else 0xFFFFFFFF.toInt()
        borderPaint.color = if (dark) 0x33FFFFFF else 0x22000000
        textPaint.color = if (dark) 0xFFFFFFFF.toInt() else 0xFF111111.toInt()

        drawRect.set(frame.left, frame.top, frame.right, frame.bottom)
        val radius = 10f * density
        canvas.drawRoundRect(drawRect, radius, radius, backgroundPaint)
        canvas.drawRoundRect(drawRect, radius, radius, borderPaint)
        val baseline = drawRect.centerY() - (textPaint.descent() + textPaint.ascent()) / 2f
        canvas.drawText(label, drawRect.centerX(), baseline, textPaint)
    }

    private fun isDark(): Boolean =
        (resources.configuration.uiMode and
            android.content.res.Configuration.UI_MODE_NIGHT_MASK) ==
            android.content.res.Configuration.UI_MODE_NIGHT_YES

    private companion object {
        const val HIDE_FADE_MS = 45L
    }
}
