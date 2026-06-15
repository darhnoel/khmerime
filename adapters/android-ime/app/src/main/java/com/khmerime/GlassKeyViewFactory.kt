package com.khmerime

import android.content.Context
import android.view.View

class GlassKeyViewFactory : KeyViewFactory {
    override fun makeKeyView(context: Context, key: KeyboardKey, onClick: () -> Unit): View =
        GlassKeyView(context, key, onClick)
}
