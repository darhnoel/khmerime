package com.khmerime.input

import android.text.InputType
import android.view.inputmethod.EditorInfo

// EnterBehavior
// =============
// What the Enter/Return key should do in the current host field, resolved from
// the field's Editor Action (see CONTEXT.md "Editor Action"). The Android-
// specific decision (imeOptions action vs. multiline / NO_ENTER_ACTION) is made
// in KhmerInputMethodService; the pure-Kotlin handler just applies the result.

sealed interface EnterBehavior {
    // Multiline field — Enter inserts a literal newline.
    object Newline : EnterBehavior

    // Single-line field with no declared action (or a suppressed one) — Enter
    // sends a real KEYCODE_ENTER key event. This both submits in-app search bars
    // that listen for the Enter key and inserts a newline where appropriate,
    // unlike a committed "\n" (which such fields ignore).
    object SendEnterKey : EnterBehavior

    // The field requests an Editor Action (Search / Go / Send / Done / Next …).
    // actionId is the Android imeOptions action id, applied opaquely by the
    // handler via TextProxy.performEditorAction.
    data class PerformAction(val actionId: Int) : EnterBehavior
}

// Resolves the Enter behavior from a field's EditorInfo, by precedence:
//   1. A declared action (Search/Go/Send/…) that is NOT suppressed -> perform it.
//      The action WINS over the multiline flag: a field that genuinely wants a
//      newline sets IME_FLAG_NO_ENTER_ACTION (TextView does this automatically for
//      multi-line views), so "has action && !suppressed" means the app wants the
//      action on Enter even if the field is also marked multi-line.
//   2. Multiline (and no usable action) -> insert a newline.
//   3. Single-line, no action -> send a real KEYCODE_ENTER key event.
// Pure function of the two ints so it can be unit-tested without a real field.
fun resolveEnterBehavior(imeOptions: Int, inputType: Int): EnterBehavior {
    val action = imeOptions and EditorInfo.IME_MASK_ACTION
    val suppressed = (imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION) != 0
    val multiline = (inputType and InputType.TYPE_TEXT_FLAG_MULTI_LINE) != 0
    val hasAction = action != EditorInfo.IME_ACTION_NONE && action != EditorInfo.IME_ACTION_UNSPECIFIED
    return when {
        hasAction && !suppressed -> EnterBehavior.PerformAction(action)
        multiline -> EnterBehavior.Newline
        else -> EnterBehavior.SendEnterKey
    }
}
