package com.khmerime

import com.khmerime.dashboard.CharacterTableFragment
import org.junit.Assert.assertEquals
import org.junit.Test

// romanChips must map each Khmer character to exactly one chip label, with a
// character's alternative spellings joined by "/" inside its own chip.
class RomanChipsTest {

    @Test
    fun consonantRowSplitsPositionallyOnePerChar() {
        assertEquals(
            listOf("k", "kh", "g", "gh", "ng"),
            CharacterTableFragment.romanChips("ក ខ គ ឃ ង", "k kh g gh ng"),
        )
    }

    @Test
    fun consonantAlternativesStayInsideTheirOwnChip() {
        // វ has two spellings; they must join inside one chip, not leak into neighbours.
        // Real data uses comma+space ("v, w"); the space inside must NOT start a new chip.
        assertEquals(
            listOf("y", "r", "l", "v/w"),
            CharacterTableFragment.romanChips("យ រ ល វ", "y r l v, w"),
        )
        assertEquals(
            listOf("s", "h", "l", "a/e/i/o/u"),
            CharacterTableFragment.romanChips("ស ហ ឡ អ", "s h l a, e, i, o, u"),
        )
    }

    @Test
    fun singleCharVowelRowIsOneChipOfAllAlternatives() {
        assertEquals(
            listOf("a/ar/ea"),
            CharacterTableFragment.romanChips("កា", "a, ar, ea"),
        )
        assertEquals(
            listOf("er"),
            CharacterTableFragment.romanChips("កើ", "er"),
        )
    }
}
