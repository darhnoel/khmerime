package com.khmerime.input

import android.content.Context

// SmartModePreference
// ===================
// Persists the user's Standard/Smart choice. Standard = lexicon + fuzzy only (the default). Smart
// enables the injected span-proposal provider inside the decoder — inert (a no-op) unless a provider
// is registered, so in the OSS build this toggle has no visible effect and the engine stays Standard.
// Mirrors the iOS SmartModePreference. Provider-agnostic: names no model.

object SmartModePreference {

    private const val PREFS_NAME = "khmerime_settings"
    private const val KEY_SMART_MODE = "smart_mode"

    fun isEnabled(context: Context): Boolean =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .getBoolean(KEY_SMART_MODE, false)

    fun setEnabled(context: Context, enabled: Boolean) {
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_SMART_MODE, enabled)
            .apply()
    }
}
