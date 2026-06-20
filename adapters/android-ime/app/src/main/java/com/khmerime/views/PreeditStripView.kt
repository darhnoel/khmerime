package com.khmerime.views

import com.khmerime.input.KhmerRenderState
import com.khmerime.layout.KeyboardPresentationSpec
import android.content.Context
import android.graphics.Paint
import android.graphics.Typeface
import android.util.AttributeSet
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView

class PreeditStripView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
) : LinearLayout(context, attrs) {

    private val romanRow = TextView(context)
    private val khmerRow = LinearLayout(context)
    private val khmerRowPool = ViewPool<TextView>(
        createChild = {
            TextView(context).apply {
                setPadding(8.dp, 0, 8.dp, 0)
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                )
            }
        },
        addChild = { khmerRow.addView(it) },
        setVisible = { view, visible -> view.visibility = if (visible) View.VISIBLE else View.GONE },
    )

    var onSegmentFocused: ((Int) -> Unit)? = null

    init {
        orientation = VERTICAL
        addView(romanRow, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT,
        ).also { it.topMargin = 4.dp })
        addView(khmerRow, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT,
        ).also { it.bottomMargin = 4.dp })
        romanRow.textSize = 12f
        romanRow.setTextColor(secondaryTextColor())
        romanRow.gravity = Gravity.CENTER
        romanRow.setSingleLine(true)
        khmerRow.orientation = HORIZONTAL
        khmerRow.gravity = Gravity.CENTER
    }

    fun render(state: KhmerRenderState, romanHint: String) {
        romanRow.text = KeyboardPresentationSpec.romanRowText(state, romanHint)
        renderKhmerRow(state)
    }

    fun clear() {
        romanRow.text = ""
        khmerRowPool.sync(0)
    }

    private fun renderKhmerRow(state: KhmerRenderState) {
        val texts = KeyboardPresentationSpec.segmentKhmerTexts(state)
        val focusedIdx = KeyboardPresentationSpec.focusedSegmentIndex(state)

        if (texts.isEmpty()) {
            val candidate = state.candidates.getOrNull(state.selectedIndex ?: 0) ?: ""
            if (candidate.isEmpty()) {
                khmerRowPool.sync(0)
            } else {
                styleCandidateLabel(khmerRowPool.sync(1)[0], candidate)
            }
        } else {
            val labels = khmerRowPool.sync(texts.size)
            texts.forEachIndexed { idx, text ->
                styleSegmentLabel(labels[idx], text, idx, focused = idx == focusedIdx)
            }
        }
    }

    private fun styleSegmentLabel(label: TextView, text: String, index: Int, focused: Boolean) {
        label.text = text
        label.setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
        label.setTextColor(if (focused) primaryTextColor() else secondaryTextColor())
        label.setTypeface(label.typeface, if (focused) Typeface.BOLD else Typeface.NORMAL)
        label.paintFlags = if (focused) {
            label.paintFlags or Paint.UNDERLINE_TEXT_FLAG
        } else {
            label.paintFlags and Paint.UNDERLINE_TEXT_FLAG.inv()
        }
        label.setOnClickListener { onSegmentFocused?.invoke(index) }
    }

    private fun styleCandidateLabel(label: TextView, text: String) {
        label.text = text
        label.setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
        label.setTextColor(primaryTextColor())
        label.setTypeface(label.typeface, Typeface.BOLD)
        label.paintFlags = label.paintFlags and Paint.UNDERLINE_TEXT_FLAG.inv()
        // A single word has no segments, so tapping it commits the shown candidate
        // directly (the handler's focusSegment treats no-segments as commit).
        label.setOnClickListener { onSegmentFocused?.invoke(0) }
    }

    private fun isDark(): Boolean =
        (resources.configuration.uiMode and
                android.content.res.Configuration.UI_MODE_NIGHT_MASK) ==
                android.content.res.Configuration.UI_MODE_NIGHT_YES

    private fun primaryTextColor(): Int =
        if (isDark()) 0xFFFFFFFF.toInt() else 0xFF111111.toInt()

    private fun secondaryTextColor(): Int =
        if (isDark()) 0xFFAAAAAA.toInt() else 0xFF666666.toInt()

    private val Int.dp: Int
        get() = (this * resources.displayMetrics.density).toInt()
}
