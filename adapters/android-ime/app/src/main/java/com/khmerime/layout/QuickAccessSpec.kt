package com.khmerime.layout

data class QuickAccessItem(
    val displayText: String,
    val commitText: String = displayText,
    val accessibilityLabel: String? = null,
)

object QuickAccessSpec {
    val digits: List<QuickAccessItem> =
        "១២៣៤៥៦៧៨៩០".map { QuickAccessItem(it.toString()) }

    val marks: List<QuickAccessItem> = listOf(
        QuickAccessItem("។", accessibilityLabel = "Khmer full stop"),
        QuickAccessItem("៕", accessibilityLabel = "Khmer final period"),
        QuickAccessItem("៖", accessibilityLabel = "Khmer sign camnuc pii kuuh"),
        QuickAccessItem("ៈ", accessibilityLabel = "Yukaleapintu"),
        QuickAccessItem("ៗ", accessibilityLabel = "Khmer repetition sign"),
        QuickAccessItem("៘"),
        QuickAccessItem("៙"),
        QuickAccessItem("៚"),
        QuickAccessItem("៛", accessibilityLabel = "Khmer currency symbol riel"),
        QuickAccessItem("៊"),
        combining("័"),
        combining("៌"),
        combining("៍"),
        combining("៏"),
        combining("៎"),
        combining("៑"),
    )

    private fun combining(mark: String): QuickAccessItem =
        QuickAccessItem(displayText = "◌$mark", commitText = mark)
}
