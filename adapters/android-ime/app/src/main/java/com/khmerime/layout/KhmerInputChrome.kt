package com.khmerime.layout

import com.khmerime.input.KhmerRenderState
import com.khmerime.input.KeyboardState

enum class KhmerInputChromePresentation(val rowCount: Int) {
    Hidden(0),
    CharPickQuickAccess(1),
    CharPickCandidates(1),
    QuickAccess(2),
    Composition(2),
}

object KhmerInputChrome {
    fun presentation(
        keyboardState: KeyboardState?,
        romanHint: String,
        state: KhmerRenderState,
    ): KhmerInputChromePresentation {
        if (keyboardState == KeyboardState.English) {
            return KhmerInputChromePresentation.Hidden
        }
        if (keyboardState == KeyboardState.SuggestCharacter) {
            return if (state.candidates.isEmpty()) {
                KhmerInputChromePresentation.CharPickQuickAccess
            } else {
                KhmerInputChromePresentation.CharPickCandidates
            }
        }
        val isComposing =
            romanHint.isNotEmpty() || state.segments.isNotEmpty() || state.preedit.isNotEmpty()
        return if (isComposing) {
            KhmerInputChromePresentation.Composition
        } else {
            KhmerInputChromePresentation.QuickAccess
        }
    }
}
