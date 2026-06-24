package com.khmerime

import android.text.InputType
import android.view.inputmethod.EditorInfo
import com.khmerime.input.EnterBehavior
import com.khmerime.input.resolveEnterBehavior
import org.junit.Assert.assertEquals
import org.junit.Test

// EnterBehaviorResolutionTest
// ===========================
// Locks in how a field's EditorInfo maps to Enter behavior (see CONTEXT.md
// "Editor Action"). Pure logic — no Android framework objects, just int flags.

class EnterBehaviorResolutionTest {

    @Test
    fun searchActionSingleLinePerformsAction() {
        val behavior = resolveEnterBehavior(EditorInfo.IME_ACTION_SEARCH, InputType.TYPE_CLASS_TEXT)
        assertEquals(EnterBehavior.PerformAction(EditorInfo.IME_ACTION_SEARCH), behavior)
    }

    @Test
    fun searchActionWithMultilineFlagStillPerformsAction() {
        // The real-world bug: a search bar that also sets TYPE_TEXT_FLAG_MULTI_LINE
        // without IME_FLAG_NO_ENTER_ACTION. The declared action must win.
        val behavior = resolveEnterBehavior(
            EditorInfo.IME_ACTION_SEARCH,
            InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE,
        )
        assertEquals(
            "a declared, non-suppressed action wins over the multiline flag",
            EnterBehavior.PerformAction(EditorInfo.IME_ACTION_SEARCH),
            behavior,
        )
    }

    @Test
    fun multilineWithNoActionInsertsNewline() {
        val behavior = resolveEnterBehavior(
            0,
            InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE,
        )
        assertEquals(EnterBehavior.Newline, behavior)
    }

    @Test
    fun multilineActionSuppressedByNoEnterActionInsertsNewline() {
        // TextView auto-sets IME_FLAG_NO_ENTER_ACTION for fields that truly want a
        // newline — those must NOT perform the action.
        val behavior = resolveEnterBehavior(
            EditorInfo.IME_ACTION_SEARCH or EditorInfo.IME_FLAG_NO_ENTER_ACTION,
            InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE,
        )
        assertEquals(EnterBehavior.Newline, behavior)
    }

    @Test
    fun singleLineNoActionSendsRealEnterKey() {
        val behavior = resolveEnterBehavior(0, InputType.TYPE_CLASS_TEXT)
        assertEquals(EnterBehavior.SendEnterKey, behavior)
    }

    @Test
    fun nullFieldSendsRealEnterKey() {
        // TYPE_NULL (e.g. a WebView) — no class, no action.
        val behavior = resolveEnterBehavior(0, 0)
        assertEquals(EnterBehavior.SendEnterKey, behavior)
    }
}
