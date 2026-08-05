package com.khmerime.views

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.util.TypedValue
import android.view.MotionEvent
import android.view.View

class SuggestionChipView(context: Context) : View(context) {

    private var text: String = ""
    private var isSelected: Boolean = false
    private var fromModel: Boolean = false
    private var lexiconVerified: Boolean = true
    private var onClick: () -> Unit = {}

    // Re-styles a pooled chip for its new candidate/selection state instead of allocating a new
    // view. ADR-0016: a model phrase (`fromModel`) shows a ✦; it's drawn RED only when the phrase
    // is NOT lexicon-verified (out-of-Lexicon = unverified trust warning), else the normal color.
    fun update(
        text: String,
        isSelected: Boolean,
        fromModel: Boolean = false,
        lexiconVerified: Boolean = true,
        onClick: () -> Unit,
    ) {
        this.text = text
        this.isSelected = isSelected
        this.fromModel = fromModel
        this.lexiconVerified = lexiconVerified
        this.onClick = onClick
        // New text may be wider/narrower — re-measure so the box fits the whole phrase.
        requestLayout()
        invalidate()
    }

    // Size the box to its text (+ breathing room), with a minimum so short word
    // candidates stay uniformly tappable. A whole-phrase card therefore extends to
    // cover the full phrase instead of clipping at a fixed width.
    // Red ✦ marker (ADR-0016): drawn before the label when the model contributed to this phrase.
    private val markerGlyph = "✦ "

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val markerWidth = if (fromModel) textPaint.measureText(markerGlyph) else 0f
        val contentWidth = markerWidth + textPaint.measureText(text) + horizontalPadding * 2
        val width = maxOf(minWidth.toFloat(), contentWidth).toInt()
        setMeasuredDimension(
            resolveSize(width, widthMeasureSpec),
            MeasureSpec.getSize(heightMeasureSpec),
        )
    }

    private val horizontalPadding: Float =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, 14f, resources.displayMetrics)
    private val minWidth: Int =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, 56f, resources.displayMetrics).toInt()

    private val isDark: Boolean
        get() = (resources.configuration.uiMode and
                android.content.res.Configuration.UI_MODE_NIGHT_MASK) ==
                android.content.res.Configuration.UI_MODE_NIGHT_YES

    private val bgPaint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val borderPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = GlassColorSpec.candidateBorderWidth()
    }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        textAlign = Paint.Align.CENTER
        textSize = TypedValue.applyDimension(
            TypedValue.COMPLEX_UNIT_SP, 18f,
            context.resources.displayMetrics,
        )
    }
    // The ✦ marker, drawn left-aligned before the label. Red iff the model phrase is unverified.
    private val markerPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        textAlign = Paint.Align.LEFT
        textSize = TypedValue.applyDimension(
            TypedValue.COMPLEX_UNIT_SP, 18f,
            context.resources.displayMetrics,
        )
    }
    private val unverifiedMarkerColor = 0xFFE53935.toInt() // material red 600 — reads on light + dark

    private val rect = RectF()
    private val cornerRadius get() = height * 0.28f

    override fun onDraw(canvas: Canvas) {
        val dark = isDark
        bgPaint.color = if (isSelected) GlassColorSpec.selectedCandidateBackground(dark)
                        else GlassColorSpec.backgroundColor(dark)
        borderPaint.color = GlassColorSpec.borderColor(dark)
        textPaint.color = if (dark) 0xFFFFFFFF.toInt() else 0xFF111111.toInt()

        rect.set(0f, 0f, width.toFloat(), height.toFloat())
        canvas.drawRoundRect(rect, cornerRadius, cornerRadius, bgPaint)
        canvas.drawRoundRect(rect, cornerRadius, cornerRadius, borderPaint)

        val baseline = height / 2f - (textPaint.descent() + textPaint.ascent()) / 2f
        if (fromModel) {
            // ✦ shows for any model phrase; it's red only when unverified (out-of-Lexicon), else the
            // normal text color. Center the [✦ + label] pair: marker left-aligned, label centered
            // in the remaining width.
            markerPaint.color = if (!lexiconVerified) unverifiedMarkerColor else textPaint.color
            val markerWidth = markerPaint.measureText(markerGlyph)
            val labelWidth = textPaint.measureText(text)
            val start = (width - markerWidth - labelWidth) / 2f
            canvas.drawText(markerGlyph, start, baseline, markerPaint)
            canvas.drawText(text, start + markerWidth + labelWidth / 2f, baseline, textPaint)
        } else {
            canvas.drawText(text, width / 2f, baseline, textPaint)
        }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (!isEnabled) return false
        when (event.actionMasked) {
            MotionEvent.ACTION_UP -> {
                if (event.x in 0f..width.toFloat() && event.y in 0f..height.toFloat()) onClick()
                return true
            }
            MotionEvent.ACTION_DOWN -> return true
        }
        return super.onTouchEvent(event)
    }
}
