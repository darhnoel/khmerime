package com.khmerime

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.view.View
import android.view.inputmethod.InputMethodManager
import androidx.appcompat.app.AppCompatActivity

class WelcomeActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_welcome)

        findViewById<View>(R.id.getStartedButton).setOnClickListener {
            markWelcomeSeen()
            startActivity(Intent(this, SetupGuideActivity::class.java))
        }

        findViewById<View>(R.id.openKeyboardLink).setOnClickListener {
            inputMethodManager()?.showInputMethodPicker()
        }
    }

    private fun markWelcomeSeen() {
        getSharedPreferences(PREFS_NAME, MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_HAS_SEEN_WELCOME, true)
            .apply()
    }

    private fun inputMethodManager(): InputMethodManager? =
        getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager

    companion object {
        const val PREFS_NAME = "khmerime_intro"
        const val KEY_HAS_SEEN_WELCOME = "has_seen_welcome"
    }
}
