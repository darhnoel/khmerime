package com.khmerime.dashboard

import com.khmerime.R

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Switch
import android.widget.TextView
import androidx.fragment.app.Fragment
import com.khmerime.input.SmartModePreference

class SettingsFragment : Fragment() {

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View? {
        return inflater.inflate(R.layout.fragment_settings, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)
        view.findViewById<TextView>(R.id.settingsVersionValue)?.text = getVersionString()

        // Standard/Smart toggle. Persisted; the IME reads it on onStartInput. Inert without a
        // registered provider (OSS build), so flipping it has no effect there.
        view.findViewById<Switch>(R.id.settingsSmartModeSwitch)?.let { toggle ->
            val ctx = view.context
            toggle.isChecked = SmartModePreference.isEnabled(ctx)
            toggle.setOnCheckedChangeListener { _, checked -> SmartModePreference.setEnabled(ctx, checked) }
        }
    }

    private fun getVersionString(): String {
        val context = context ?: return "1.0 (1)"
        val info = context.packageManager.getPackageInfo(context.packageName, 0)
        return "${info.versionName} (${info.longVersionCode})"
    }
}
