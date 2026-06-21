package com.khmerime

import android.content.Intent
import android.content.pm.PackageManager
import android.view.inputmethod.InputMethod
import com.khmerime.service.KhmerInputMethodService
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class KhmerImeServiceRegistrationTest {
    @Test
    fun appExposesKhmerInputMethodService() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val intent = Intent(InputMethod.SERVICE_INTERFACE).setPackage(context.packageName)

        @Suppress("DEPRECATION")
        val services = context.packageManager.queryIntentServices(
            intent,
            PackageManager.GET_META_DATA,
        )

        val service = services
            .map { it.serviceInfo }
            .singleOrNull { it.name == KhmerInputMethodService::class.java.name }

        assertNotNull("Khmer input method service must be registered", service)
        assertEquals(
            "IME service must require Android's input-method binding permission",
            android.Manifest.permission.BIND_INPUT_METHOD,
            service!!.permission,
        )
        assertTrue("IME service must be exported so Android can bind it", service.exported)
        assertTrue(
            "IME service must declare android.view.im metadata",
            service.metaData.containsKey("android.view.im"),
        )
    }
}
