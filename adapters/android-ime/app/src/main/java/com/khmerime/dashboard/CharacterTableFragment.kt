package com.khmerime.dashboard

import com.khmerime.R

import android.graphics.Typeface
import android.os.Bundle
import android.view.Gravity
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.core.content.ContextCompat
import androidx.fragment.app.Fragment
import androidx.recyclerview.widget.RecyclerView
import androidx.viewpager2.widget.ViewPager2
import com.google.android.material.tabs.TabLayout
import com.google.android.material.tabs.TabLayoutMediator

// CharacterTableFragment — Dashboard reference tab. A romanization lookup for every
// Khmer consonant and vowel, transcribed from data/khmer_character_table.md. One
// swipeable page per section (consonants / dependent vowels / independent vowels),
// each scrollable internally, with a dot indicator — same slider style as Tips.
class CharacterTableFragment : Fragment() {

    private data class Row(val khmer: String, val roman: String)
    private data class Section(val titleRes: Int, val rows: List<Row>)

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View {
        val ctx = requireContext()
        fun dp(v: Int) = (v * resources.displayMetrics.density).toInt()
        val ink = ContextCompat.getColor(ctx, R.color.brand_ink)

        val pager = ViewPager2(ctx).apply {
            id = View.generateViewId()
            adapter = SectionAdapter(sections) { getString(it) }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f)
        }

        val dots = TabLayout(ctx).apply {
            tabGravity = TabLayout.GRAVITY_CENTER
            setSelectedTabIndicatorHeight(0)
            setBackgroundColor(android.graphics.Color.TRANSPARENT)
        }

        val root = LinearLayout(ctx).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(ink)
            setPadding(0, dp(16), 0, dp(20))
            addView(pager)
            addView(dots, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT,
            ).apply { gravity = Gravity.CENTER_HORIZONTAL })
        }

        TabLayoutMediator(dots, pager) { _, _ -> }.attach()
        for (i in 0 until dots.tabCount) {
            dots.getTabAt(i)?.view?.let { tab ->
                tab.isClickable = false
                tab.setBackgroundResource(R.drawable.tip_dot)
                tab.minimumWidth = dp(20)
            }
        }
        return root
    }

    private class SectionAdapter(
        private val sections: List<Section>,
        private val string: (Int) -> String,
    ) : RecyclerView.Adapter<SectionViewHolder>() {

        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): SectionViewHolder {
            val ctx = parent.context
            fun dp(v: Int) = (v * ctx.resources.displayMetrics.density).toInt()
            // Fixed title on top; only the rows scroll under it.
            val titleView = TextView(ctx).apply {
                setTextColor(ContextCompat.getColor(ctx, R.color.brand_amber))
                textSize = 15f
                typeface = Typeface.DEFAULT_BOLD
                gravity = Gravity.CENTER_HORIZONTAL
                setPadding(dp(4), dp(8), dp(4), dp(14))
            }
            val column = LinearLayout(ctx).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(dp(20), 0, dp(20), dp(24))
            }
            val scroll = ScrollView(ctx).apply {
                isFillViewport = true
                isVerticalScrollBarEnabled = false
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f)
                addView(column)
            }
            val page = LinearLayout(ctx).apply {
                orientation = LinearLayout.VERTICAL
                layoutParams = ViewGroup.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
                setPadding(dp(20), dp(12), dp(20), 0)
                addView(titleView)
                addView(scroll)
            }
            return SectionViewHolder(page, titleView, column)
        }

        override fun getItemCount() = sections.size

        override fun onBindViewHolder(holder: SectionViewHolder, position: Int) {
            val ctx = holder.column.context
            fun dp(v: Int) = (v * ctx.resources.displayMetrics.density).toInt()
            val ivory = ContextCompat.getColor(ctx, R.color.brand_ivory)
            val dim = ContextCompat.getColor(ctx, R.color.brand_ivory_dim)

            val section = sections[position]
            holder.title.text = string(section.titleRes)
            holder.column.removeAllViews()
            for (row in section.rows) {
                val rowView = LinearLayout(ctx).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.CENTER_VERTICAL
                    setPadding(dp(14), dp(12), dp(14), dp(12))
                }
                rowView.addView(TextView(ctx).apply {
                    text = row.khmer
                    setTextColor(ivory)
                    textSize = 20f
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                })
                rowView.addView(TextView(ctx).apply {
                    text = row.roman
                    setTextColor(dim)
                    textSize = 14f
                    typeface = Typeface.MONOSPACE
                    gravity = Gravity.END
                })
                holder.column.addView(rowView)
            }
        }
    }

    private class SectionViewHolder(
        itemView: View,
        val title: TextView,
        val column: LinearLayout,
    ) : RecyclerView.ViewHolder(itemView)

    // Character data is the reference itself, not UI copy — kept inline (not a string resource).
    private val sections = listOf(
        Section(R.string.chars_section_consonants, listOf(
            Row("ក ខ គ ឃ ង", "k kh g gh ng"),
            Row("ច ឆ ជ ឈ ញ", "ch chh j jh nh"),
            Row("ដ ឋ ឌ ឍ ណ", "d th dd ddh n"),
            Row("ត ថ ទ ធ ន", "t th tt tth n"),
            Row("ប ផ ព ភ ម", "b bh p ph m"),
            Row("យ រ ល វ", "y r l v, w"),
            Row("ស ហ ឡ អ", "s h l a, e, i, o, u"),
        )),
        Section(R.string.chars_section_dependent_vowels, listOf(
            Row("កា", "a, ar, ea"),
            Row("កិ", "e, i"),
            Row("កី", "ei, ey"),
            Row("កឹ", "e, eu, ue, ir"),
            Row("កឺ", "e, eu, er"),
            Row("កុ", "o, u"),
            Row("កូ", "o, u, ou"),
            Row("កួ", "uo, ou"),
            Row("កើ", "er"),
            Row("កឿ", "oeu"),
            Row("កៀ", "ie"),
            Row("កេ", "e"),
            Row("កែ", "e, ae"),
            Row("កៃ", "ai, ay, ei, ey"),
            Row("កោ", "ao, ou"),
            Row("កៅ", "av, au, ov"),
            Row("កុំ", "um, om"),
            Row("កំ", "om, um"),
            Row("កាំ", "am, an-, ean, oam"),
            Row("កះ", "ah, eah, eh"),
            Row("កុះ", "oh, uh, os, us"),
            Row("កេះ", "eh, ih, es, is"),
            Row("កោះ", "oh, os, uoh, uos, ouh, ous"),
        )),
        Section(R.string.chars_section_independent_vowels, listOf(
            Row("ឥ", "ei, i, e, eu"),
            Row("ឦ", "ei, i"),
            Row("ឧ", "u"),
            Row("ឩ", "u"),
            Row("ឪ", "ov"),
            Row("ឫ", "ru, reu"),
            Row("ឬ", "ru, reu"),
            Row("ឭ", "leu"),
            Row("ឮ", "leu"),
            Row("ឯ", "ae, e"),
            Row("ឰ", "ai"),
            Row("ឱ", "ao"),
            Row("ឲ", "ao"),
            Row("ឳ", "av, aov"),
        )),
    )
}
