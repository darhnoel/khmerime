package com.khmerime

import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Test
import org.w3c.dom.Element

class WelcomeActivityManifestTest {
    @Test
    fun appLaunchesWelcomeActivityFromLauncherIcon() {
        val manifest = DocumentBuilderFactory.newInstance()
            .newDocumentBuilder()
            .parse(androidManifestFile())

        val welcomeActivity = manifest
            .getElementsByTagName("activity")
            .asElements()
            .singleOrNull { it.androidAttribute("name") == ".WelcomeActivity" }

        assertNotNull("WelcomeActivity must be declared in the app manifest", welcomeActivity)
        assertEquals(
            "WelcomeActivity must be exported so Android can launch it",
            "true",
            welcomeActivity!!.androidAttribute("exported"),
        )

        val launcherFilter = welcomeActivity
            .getElementsByTagName("intent-filter")
            .asElements()
            .singleOrNull { filter ->
                filter.hasChildWithAndroidName("action", "android.intent.action.MAIN") &&
                    filter.hasChildWithAndroidName("category", "android.intent.category.LAUNCHER")
            }

        assertNotNull("WelcomeActivity must be the launcher entry point", launcherFilter)
    }

    private fun androidManifestFile(): File {
        val candidates = listOf(
            File("src/main/AndroidManifest.xml"),
            File("app/src/main/AndroidManifest.xml"),
        )

        return candidates.firstOrNull { it.isFile }
            ?: error("Could not locate AndroidManifest.xml from ${File(".").canonicalPath}")
    }

    private fun Element.hasChildWithAndroidName(tagName: String, name: String): Boolean =
        getElementsByTagName(tagName)
            .asElements()
            .any { it.androidAttribute("name") == name }

    private fun Element.androidAttribute(name: String): String = getAttribute("android:$name")

    private fun org.w3c.dom.NodeList.asElements(): List<Element> =
        (0 until length)
            .mapNotNull { item(it) as? Element }
}
