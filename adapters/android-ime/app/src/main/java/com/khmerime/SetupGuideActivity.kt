package com.khmerime

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.view.View
import android.view.inputmethod.InputMethodManager
import androidx.appcompat.app.AppCompatActivity

class SetupGuideActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_setup_guide)

        findViewById<View>(R.id.openSettingsButton).setOnClickListener {
            startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS))
        }

        findViewById<View>(R.id.openKeyboardPickerLink).setOnClickListener {
            inputMethodManager()?.showInputMethodPicker()
        }

        findViewById<View>(R.id.continueButton).setOnClickListener {
            startActivity(Intent(this, DashboardActivity::class.java))
            finish()
        }
    }

    private fun inputMethodManager(): InputMethodManager? =
        getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
}
