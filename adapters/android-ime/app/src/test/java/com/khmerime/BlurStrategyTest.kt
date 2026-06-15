package com.khmerime

import com.khmerime.views.BlurStrategy
import org.junit.Assert.*
import org.junit.Test

class BlurStrategyTest {

    @Test
    fun api31ReturnsRenderEffectStrategy() {
        assertTrue(
            "API 31 must use RenderEffect blur",
            BlurStrategy.forApi(31) is BlurStrategy.RenderEffect,
        )
    }

    @Test
    fun api36ReturnsRenderEffectStrategy() {
        assertTrue(
            "API 36 must use RenderEffect blur",
            BlurStrategy.forApi(36) is BlurStrategy.RenderEffect,
        )
    }

    @Test
    fun api30ReturnsNoOpStrategy() {
        assertTrue(
            "API 30 must fall back to no-op blur",
            BlurStrategy.forApi(30) is BlurStrategy.NoOp,
        )
    }

    @Test
    fun api24ReturnsNoOpStrategy() {
        assertTrue(
            "API 24 (minSdk) must fall back to no-op blur",
            BlurStrategy.forApi(24) is BlurStrategy.NoOp,
        )
    }
}
