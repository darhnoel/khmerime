package com.khmerime

import com.google.gson.annotations.SerializedName

data class KhmerRenderState(
    val candidates: List<String> = emptyList(),
    @SerializedName("selected_index") val selectedIndex: Int? = null,
    val preedit: String = "",
    val segments: List<KhmerSegmentEntry> = emptyList(),
    @SerializedName("focused_segment_index") val focusedSegmentIndex: Int? = null,
    @SerializedName("commit_text") val commitText: String? = null,
    @SerializedName("segment_edit_active") val segmentEditActive: Boolean = false,
    @SerializedName("segment_edit_index") val segmentEditIndex: Int? = null,
)

data class KhmerSegmentEntry(
    val output: String = "",
    val input: String = "",
    val focused: Boolean = false,
)
