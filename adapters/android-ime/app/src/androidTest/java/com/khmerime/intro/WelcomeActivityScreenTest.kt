package com.khmerime.intro

import com.khmerime.R

import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.matcher.ViewMatchers.withId
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.rules.ActivityScenarioRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class WelcomeActivityScreenTest {
    @get:Rule
    val activityRule = ActivityScenarioRule(WelcomeActivity::class.java)

    @Before
    fun clearWelcomeState() {
        targetContext()
            .getSharedPreferences(WelcomeActivity.PREFS_NAME, 0)
            .edit()
            .clear()
            .commit()
    }

    @Test
    fun welcomeScreenPresentsBrandIntro() {
        onView(withId(R.id.logoCard)).check(matches(isDisplayed()))
        onView(withText(R.string.app_name)).check(matches(isDisplayed()))
        onView(withText(R.string.welcome_subtitle)).check(matches(isDisplayed()))
        onView(withText(R.string.welcome_get_started)).check(matches(isDisplayed()))
        onView(withText(R.string.welcome_already_enabled)).check(matches(isDisplayed()))
    }

    @Test
    fun getStartedRemembersWelcomeAndOpensSetupGuide() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val setupGuideMonitor = instrumentation.addMonitor(
            "com.khmerime.intro.SetupGuideActivity",
            null,
            false,
        )

        onView(withId(R.id.getStartedButton)).perform(click())

        val setupGuide = instrumentation.waitForMonitorWithTimeout(setupGuideMonitor, 3_000)
        assertNotNull("Get Started must open the setup guide", setupGuide)
        onView(withText("Enable KhmerIME")).check(matches(isDisplayed()))
        onView(withText("Open Settings")).check(matches(isDisplayed()))
        assertTrue(
            "Get Started must remember the welcome screen was completed",
            targetContext()
                .getSharedPreferences(WelcomeActivity.PREFS_NAME, 0)
                .getBoolean(WelcomeActivity.KEY_HAS_SEEN_WELCOME, false),
        )

        setupGuide?.finish()
    }

    private fun targetContext() = InstrumentationRegistry.getInstrumentation().targetContext
}
