package com.khmerime.views

import com.khmerime.input.KhmerRenderState
import com.khmerime.layout.KeyboardPresentationSpec
import android.content.Context
import android.graphics.Paint
import android.graphics.Typeface
import android.util.AttributeSet
import android.util.TypedValue
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.TextView

class PreeditStripView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
) : LinearLayout(context, attrs) {

    private val romanRow = TextView(context)
    private val khmerRow = LinearLayout(context)

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
        khmerRow.removeAllViews()
    }

    private fun renderKhmerRow(state: KhmerRenderState) {
        khmerRow.removeAllViews()
        val texts = KeyboardPresentationSpec.segmentKhmerTexts(state)
        val focusedIdx = KeyboardPresentationSpec.focusedSegmentIndex(state)

        if (texts.isEmpty()) {
            val candidate = state.candidates.getOrNull(state.selectedIndex ?: 0) ?: ""
            if (candidate.isNotEmpty()) khmerRow.addView(makeCandidateLabel(candidate))
        } else {
            texts.forEachIndexed { idx, text ->
                khmerRow.addView(makeSegmentLabel(text, idx, focused = idx == focusedIdx))
            }
        }
    }

    private fun makeSegmentLabel(text: String, index: Int, focused: Boolean): TextView =
        TextView(context).apply {
            this.text = text
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
            setTextColor(if (focused) primaryTextColor() else secondaryTextColor())
            if (focused) {
                setTypeface(typeface, Typeface.BOLD)
                paintFlags = paintFlags or Paint.UNDERLINE_TEXT_FLAG
            }
            setPadding(8.dp, 0, 8.dp, 0)
            setOnClickListener { onSegmentFocused?.invoke(index) }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT,
            )
        }

    private fun makeCandidateLabel(text: String): TextView =
        TextView(context).apply {
            this.text = text
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
            setTextColor(primaryTextColor())
            setTypeface(typeface, Typeface.BOLD)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT,
            )
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
