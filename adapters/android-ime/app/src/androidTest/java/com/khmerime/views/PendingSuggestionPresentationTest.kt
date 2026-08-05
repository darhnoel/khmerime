package com.khmerime.views

import android.view.ViewGroup
import android.widget.TextView
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.khmerime.input.KhmerRenderState
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class PendingSuggestionPresentationTest {
    @Test
    fun pendingDecodeUpdatesRomanWithoutHidingOrActivatingStaleKhmer() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()

        instrumentation.runOnMainSync {
            var taps = 0
            val strip = PreeditStripView(instrumentation.targetContext).apply {
                onSegmentFocused = { taps += 1 }
                render(
                    KhmerRenderState(
                        candidates = listOf("ខ្ញុំ"),
                        selectedIndex = 0,
                        preedit = "nhom",
                    ),
                    romanHint = "nhom",
                )
            }
            val romanRow = strip.getChildAt(0) as TextView
            val khmerRow = strip.getChildAt(1) as ViewGroup
            val previousSuggestion = khmerRow.getChildAt(0) as TextView

            strip.showPendingRoman("nhomtt")

            assertEquals("nhomtt", romanRow.text.toString())
            assertEquals("ខ្ញុំ", previousSuggestion.text.toString())
            previousSuggestion.performClick()
            assertEquals("a stale suggestion must not act on the newer composition", 0, taps)
        }
    }
}
