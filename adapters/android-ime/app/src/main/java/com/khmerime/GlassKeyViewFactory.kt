package com.khmerime

import android.content.Context
import android.view.View

class GlassKeyViewFactory(
    private val keyboardState: KeyboardState = KeyboardState.Qwerty,
) : KeyViewFactory {
    override fun makeKeyView(context: Context, key: KeyboardKey, onClick: () -> Unit): View =
        GlassKeyView(
            context = context,
            key = key,
            isActive = KeyboardPresentationSpec.isToggleActive(key, keyboardState),
            onClick = onClick,
        )
}
