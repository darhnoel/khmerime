package com.khmerime.dashboard

import com.khmerime.R

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.TextView
import androidx.fragment.app.Fragment
import androidx.recyclerview.widget.RecyclerView
import androidx.viewpager2.widget.ViewPager2
import com.google.android.material.tabs.TabLayout
import com.google.android.material.tabs.TabLayoutMediator

// TipsFragment
// ============
// Swipeable tips carousel (iOS TipsViewController parity): a horizontal ViewPager2
// of tip pages with a TabLayout dot indicator, replacing the old vertical scroll.
class TipsFragment : Fragment() {

    private data class Tip(val icon: Int, val headline: Int, val body: Int)

    private val tips = listOf(
        Tip(R.drawable.ic_tip_keyboard, R.string.tip_1_headline, R.string.tip_1_body),
        Tip(R.drawable.ic_tip_text, R.string.tip_2_headline, R.string.tip_2_body),
        Tip(R.drawable.ic_tip_character, R.string.tip_3_headline, R.string.tip_3_body),
        Tip(R.drawable.ic_tip_globe, R.string.tip_4_headline, R.string.tip_4_body),
    )

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View =
        inflater.inflate(R.layout.fragment_tips, container, false)

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        val pager = view.findViewById<ViewPager2>(R.id.tips_pager)
        val dots = view.findViewById<TabLayout>(R.id.tips_dots)
        pager.adapter = TipAdapter(tips)
        // Attach dots to the pager; tabs are indicators only (not tappable).
        TabLayoutMediator(dots, pager) { _, _ -> }.attach()
        for (i in 0 until dots.tabCount) dots.getTabAt(i)?.view?.isClickable = false
    }

    private class TipAdapter(private val tips: List<Tip>) : RecyclerView.Adapter<TipViewHolder>() {
        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): TipViewHolder {
            val v = LayoutInflater.from(parent.context).inflate(R.layout.item_tip_page, parent, false)
            return TipViewHolder(v)
        }

        override fun getItemCount() = tips.size

        override fun onBindViewHolder(holder: TipViewHolder, position: Int) {
            val tip = tips[position]
            holder.icon.setImageResource(tip.icon)
            holder.headline.setText(tip.headline)
            holder.body.setText(tip.body)
        }
    }

    private class TipViewHolder(view: View) : RecyclerView.ViewHolder(view) {
        val icon: ImageView = view.findViewById(R.id.tip_icon)
        val headline: TextView = view.findViewById(R.id.tip_headline)
        val body: TextView = view.findViewById(R.id.tip_body)
    }
}
