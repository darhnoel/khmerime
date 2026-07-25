package com.khmerime.layout

// Kotlin port of the iOS QwertyCharacterGridLayout struct. Produces the staggered
// iOS look: letter keys keep a constant width across all three rows, row 2
// (asdfghjkl, 9 keys) is centered with side insets, and row 3's edge controls (✦/⌫)
// widen to fill the space around 7 constant-width letters.
//
// availableWidth = the row's usable width; spacing = the inter-key gap.
class QwertyCharacterGridLayout(
    private val availableWidth: Float,
    private val spacing: Float,
) {
    // Row 1 baseline: 10 letters + 9 gaps fill the width.
    val characterKeyWidth: Float
        get() = (availableWidth - spacing * 9) / 10

    // Row 2 (9 keys): center the constant-width keys, splitting the leftover evenly.
    val row2SideInset: Float
        get() = (availableWidth - characterKeyWidth * 9 - spacing * 8) / 2

    // Row 3 (✦ + 7 letters + ⌫): each edge control fills half the space left around
    // 7 constant-width letters and their 8 gaps.
    val row3ControlWidth: Float
        get() = (availableWidth - characterKeyWidth * 7 - spacing * 8) / 2
}
