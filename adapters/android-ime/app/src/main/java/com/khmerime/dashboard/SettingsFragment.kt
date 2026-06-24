package com.khmerime.dashboard

import com.khmerime.R

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.fragment.app.Fragment

class SettingsFragment : Fragment() {

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View? {
        return inflater.inflate(R.layout.fragment_settings, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)
        view.findViewById<TextView>(R.id.settingsVersionValue)?.text = getVersionString()
    }

    private fun getVersionString(): String {
        val context = context ?: return "1.0 (1)"
        val info = context.packageManager.getPackageInfo(context.packageName, 0)
        return "${info.versionName} (${info.longVersionCode})"
    }
}
