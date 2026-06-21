package com.khmerime

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.fragment.app.Fragment
import com.google.android.material.bottomnavigation.BottomNavigationView

class DashboardActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_dashboard)

        val bottomNav = findViewById<BottomNavigationView>(R.id.bottomNav)
        bottomNav.setOnItemSelectedListener { item ->
            when (item.itemId) {
                R.id.nav_settings -> switchFragment(SettingsFragment())
                R.id.nav_tips -> switchFragment(TipsFragment())
                R.id.nav_support -> switchFragment(SupportFragment())
            }
            true
        }

        if (savedInstanceState == null) {
            switchFragment(SettingsFragment())
        }
    }

    private fun switchFragment(fragment: Fragment) {
        supportFragmentManager.beginTransaction()
            .replace(R.id.dashboardContent, fragment)
            .commit()
    }
}
