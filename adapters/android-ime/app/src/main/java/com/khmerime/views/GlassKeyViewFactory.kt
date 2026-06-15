package com.khmerime.views

import com.khmerime.input.KeyboardState
import com.khmerime.layout.KeyboardKey
import com.khmerime.layout.KeyboardPresentationSpec
import com.khmerime.layout.KeyViewFactory
import android.content.Context
import android.view.View

class GlassKeyViewFactory(
    private val keyboardState: KeyboardState = KeyboardState.Qwerty,
) : KeyViewFactory {
    override fun makeKeyView(context: Context, key: KeyboardKey, onClick: () -> Unit): View =
        GlassKeyView(
            context = context,
            key = key,
            isActive = KeyboardPresentationSpec.isKeyActive(key, keyboardState),
            onClick = onClick,
        )
}
