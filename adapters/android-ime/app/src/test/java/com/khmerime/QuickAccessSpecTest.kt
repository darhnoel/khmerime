package com.khmerime

import com.khmerime.layout.QuickAccessSpec
import org.junit.Assert.assertEquals
import org.junit.Test

class QuickAccessSpecTest {
    @Test
    fun khmerQuickAccessOwnsExactDigitsMarksAndDisplayOnlyDottedCircles() {
        assertEquals("១២៣៤៥៦៧៨៩០", QuickAccessSpec.digits.joinToString("") { it.commitText })
        assertEquals(
            listOf("។", "៕", "៖", "ៈ", "ៗ", "៘", "៙", "៚", "៛", "៊", "័", "៌", "៍", "៏", "៎", "៑"),
            QuickAccessSpec.marks.map { it.commitText },
        )
        assertEquals(
            listOf("។", "៕", "៖", "ៈ", "ៗ", "៘", "៙", "៚", "៛", "៊", "◌័", "◌៌", "◌៍", "◌៏", "◌៎", "◌៑"),
            QuickAccessSpec.marks.map { it.displayText },
        )
    }
}
