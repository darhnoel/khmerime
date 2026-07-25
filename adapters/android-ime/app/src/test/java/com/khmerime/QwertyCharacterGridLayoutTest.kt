package com.khmerime

import com.khmerime.layout.QwertyCharacterGridLayout
import org.junit.Assert.assertEquals
import org.junit.Test

// Mirrors the iOS QwertyCharacterGridLayout struct: constant letter-key width across
// all three rows, row 2 (asdfghjkl) centered with side insets, row 3 with wide edge
// controls. These are the geometry numbers that produce the staggered iOS look.
class QwertyCharacterGridLayoutTest {

    // Row 1 is the baseline: 10 letter keys + 9 inter-key gaps exactly fill the width.
    @Test
    fun characterKeyWidthFillsTenKeysAndNineGaps() {
        val layout = QwertyCharacterGridLayout(availableWidth = 360f, spacing = 6f)
        // (360 - 6*9) / 10 = (360 - 54) / 10 = 30.6
        assertEquals(30.6f, layout.characterKeyWidth, 0.001f)
    }

    // Row 2 (asdfghjkl) keeps the SAME 9 constant-width keys but centers them:
    // the leftover width splits evenly into a leading + trailing side inset.
    @Test
    fun row2SideInsetCentersNineConstantWidthKeys() {
        val layout = QwertyCharacterGridLayout(availableWidth = 360f, spacing = 6f)
        // keyW=30.6; row2 content = 30.6*9 + 6*8 = 275.4 + 48 = 323.4; leftover = 36.6; ÷2 = 18.3
        assertEquals(18.3f, layout.row2SideInset, 0.001f)
    }

    // Row 3 (✦ + 7 letters + ⌫): the two edge controls each widen to fill the space
    // left around 7 constant-width letters (and their 8 gaps), split evenly.
    @Test
    fun row3ControlWidthFillsAroundSevenLetters() {
        val layout = QwertyCharacterGridLayout(availableWidth = 360f, spacing = 6f)
        // keyW=30.6; (360 - 30.6*7 - 6*8) / 2 = (360 - 214.2 - 48) / 2 = 97.8 / 2 = 48.9
        assertEquals(48.9f, layout.row3ControlWidth, 0.001f)
    }
}
