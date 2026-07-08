package com.khmerime

import com.khmerime.input.KhmerImeSession
import com.khmerime.input.KhmerRenderState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class KhmerImeSessionContractTest {
    @Test
    fun twoWordInputProducesSegmentEntries() {
        val session = KhmerImeSession()
        session.focusIn()

        var state = KhmerRenderState()
        for (ch in "khnhomtov") {
            state = session.processCharacter(ch.toString())
        }

        assertTrue(
            "khnhomtov must produce multiple segment entries; got ${state.segments}",
            state.segments.size >= 2,
        )
        assertTrue(
            "one segment should own candidate focus",
            state.segments.any { it.focused },
        )
    }

    @Test
    fun processDigitSelectsCandidateThroughSession() {
        val session = KhmerImeSession()
        session.focusIn()

        var state = KhmerRenderState()
        for (ch in "nhom") {
            state = session.processCharacter(ch.toString())
        }

        assertTrue("fixture must expose at least one candidate", state.candidates.isNotEmpty())

        val selected = session.processDigit(1)

        assertTrue(
            "digit selection should keep or produce candidates through the session",
            selected.candidates.isNotEmpty() || !selected.commitText.isNullOrEmpty(),
        )
    }

    @Test
    fun charPickModeMapsRomanLetterToKhmerCharacterCandidates() {
        val session = KhmerImeSession()
        session.focusIn()

        session.enterCharPick()
        val state = session.processCharacter("k")

        assertTrue(
            "CharPick candidates for k must include ក; got ${state.candidates}",
            state.candidates.contains("ក"),
        )

        val romanState = session.exitCharPick()
        assertTrue(
            "exiting CharPick must leave a clean roman render state",
            romanState.candidates.isEmpty() && romanState.preedit.isEmpty(),
        )
    }

    @Test
    fun phraseCandidatesExposeKhmerWholePhraseAlternatives() {
        val session = KhmerImeSession()
        session.focusIn()

        var state = KhmerRenderState()
        // "khnhom" has multiple Khmer readings (ខ្ញុំ / ខ្ញំ) → >= 2 phrase candidates.
        for (ch in "khnhom") {
            state = session.processCharacter(ch.toString())
        }

        assertTrue(
            "khnhom must expose >= 2 phrase candidates for the wheel; got ${state.phraseCandidates}",
            state.phraseCandidates.size >= 2,
        )
        assertTrue(
            "the top phrase candidate carries its segmentation",
            state.phraseCandidates.firstOrNull()?.segments?.isNotEmpty() == true,
        )
    }

    @Test
    fun selectPhraseMakesEnterCommitThatReading() {
        val session = KhmerImeSession()
        session.focusIn()

        var state = KhmerRenderState()
        for (ch in "khnhom") {
            state = session.processCharacter(ch.toString())
        }
        val wanted = state.phraseCandidates[1].text

        session.selectPhrase(1)
        val committed = session.processEnter().commitText

        assertEquals("selecting phrase 1 then Enter commits that reading", wanted, committed)
    }
}
