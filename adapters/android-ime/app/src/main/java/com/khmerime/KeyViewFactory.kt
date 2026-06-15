package com.khmerime

import android.content.Context
import android.view.View
import android.view.ContextThemeWrapper
import android.widget.Button

interface KeyViewFactory {
    fun makeKeyView(context: Context, key: KeyboardKey, onClick: () -> Unit): View
}

class DefaultKeyViewFactory : KeyViewFactory {
    override fun makeKeyView(context: Context, key: KeyboardKey, onClick: () -> Unit): View {
        val style = if (KeyViewStyle.isAction(key)) R.style.ActionButton else R.style.KeyButton
        return Button(ContextThemeWrapper(context, style)).apply {
            text = key.label
            isAllCaps = false
            textSize = if (key.label.length > 1) 13f else 16f
            setOnClickListener { onClick() }
        }
    }
}
