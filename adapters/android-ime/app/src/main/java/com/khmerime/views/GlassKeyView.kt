package com.khmerime

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.util.TypedValue
import android.view.MotionEvent
import android.view.View

class GlassKeyView(
    context: Context,
    private val key: KeyboardKey,
    private val isActive: Boolean = false,
    private val onClick: () -> Unit,
) : View(context) {

    private val isDark: Boolean
        get() = (resources.configuration.uiMode and
                android.content.res.Configuration.UI_MODE_NIGHT_MASK) ==
                android.content.res.Configuration.UI_MODE_NIGHT_YES

    private val bgPaint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val borderPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE; strokeWidth = 1f }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER }

    private val rect = RectF()
    private val cornerRadius get() = height * 0.22f

    private val squishScale get() = 1f - squish * 0.08f

    private var squish = 0f

    private val animator = GlassKeyPressAnimator(onUpdate = { s ->
        squish = s
        scaleX = squishScale
        scaleY = squishScale
    })

    override fun onDraw(canvas: Canvas) {
        val dark = isDark
        bgPaint.color = if (isActive) GlassColorSpec.toggleActiveBackground(dark)
                        else GlassColorSpec.backgroundColor(dark)
        borderPaint.color = GlassColorSpec.borderColor(dark)
        textPaint.color = if (isActive) GlassColorSpec.toggleActiveTextColor()
                          else if (dark) 0xFFFFFFFF.toInt() else 0xFF111111.toInt()
        val spSize = if (key.label.length > 1) 13f else 16f
        textPaint.textSize = TypedValue.applyDimension(
            TypedValue.COMPLEX_UNIT_SP, spSize, resources.displayMetrics,
        )

        rect.set(0f, 0f, width.toFloat(), height.toFloat())
        canvas.drawRoundRect(rect, cornerRadius, cornerRadius, bgPaint)
        canvas.drawRoundRect(rect, cornerRadius, cornerRadius, borderPaint)
        canvas.drawText(key.label, width / 2f, height / 2f - (textPaint.descent() + textPaint.ascent()) / 2f, textPaint)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                animator.press()
                return true
            }
            MotionEvent.ACTION_UP -> {
                animator.release()
                if (event.x in 0f..width.toFloat() && event.y in 0f..height.toFloat()) onClick()
                return true
            }
            MotionEvent.ACTION_CANCEL -> {
                animator.release()
                return true
            }
        }
        return super.onTouchEvent(event)
    }
}
