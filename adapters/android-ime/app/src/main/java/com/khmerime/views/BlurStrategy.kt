package com.khmerime.views

import android.os.Build
import android.view.View
import androidx.annotation.RequiresApi

sealed interface BlurStrategy {
    fun apply(view: View, radiusPx: Float)

    object NoOp : BlurStrategy {
        override fun apply(view: View, radiusPx: Float) = Unit
    }

    @RequiresApi(31)
    object RenderEffect : BlurStrategy {
        override fun apply(view: View, radiusPx: Float) {
            view.setRenderEffect(
                android.graphics.RenderEffect.createBlurEffect(
                    radiusPx, radiusPx,
                    android.graphics.Shader.TileMode.CLAMP,
                )
            )
        }
    }

    companion object {
        fun forApi(apiLevel: Int): BlurStrategy =
            if (apiLevel >= 31) RenderEffect else NoOp

        fun current(): BlurStrategy = forApi(Build.VERSION.SDK_INT)
    }
}
