package com.khmerime

import android.view.Gravity
import android.view.LayoutInflater
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class CandidateStripLayoutTest {
    @Test
    fun candidateGroupsCenterWhenTheyFitWithoutChangingChipSpacing() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val root = LayoutInflater.from(context).inflate(R.layout.keyboard, null)
        val scroll = root.findViewById<HorizontalScrollView>(R.id.candidate_scroll)
        val strip = root.findViewById<LinearLayout>(R.id.candidate_strip)

        assertTrue("the scroll viewport must give sparse content the full row width", scroll.isFillViewport)
        assertEquals(Gravity.CENTER_HORIZONTAL, strip.gravity and Gravity.HORIZONTAL_GRAVITY_MASK)
    }
}
