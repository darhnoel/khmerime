package com.example.khmerime

import com.google.gson.annotations.SerializedName

data class KhmerRenderState(
    val candidates: List<String> = emptyList(),
    @SerializedName("selected_index") val selectedIndex: Int? = null,
    val preedit: String = "",
    @SerializedName("commit_text") val commitText: String? = null,
)
