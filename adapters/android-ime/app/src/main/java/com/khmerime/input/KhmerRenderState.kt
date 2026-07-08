package com.khmerime.input

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
    @SerializedName("phrase_candidates") val phraseCandidates: List<KhmerPhraseCandidate> = emptyList(),
    @SerializedName("selected_phrase_index") val selectedPhraseIndex: Int = 0,
)

data class KhmerSegmentEntry(
    val output: String = "",
    val input: String = "",
    val focused: Boolean = false,
)

data class KhmerPhraseCandidate(
    val text: String = "",
    val segments: List<KhmerSegmentEntry> = emptyList(),
)
