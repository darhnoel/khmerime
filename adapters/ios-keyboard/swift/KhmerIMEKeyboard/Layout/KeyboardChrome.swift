import Foundation

// KeyboardChrome
// ==============
// Decides whether the input chrome (strip + candidate row) is worth its height.
// The two rows reserve 88pt, so the keyboard collapses to keys-only when idle and
// expands while composing — reclaiming screen space when there's nothing to show.
//
// Content-driven, not state-driven: focusIn renders an empty state (collapsed),
// while CharPick can populate candidates with an empty strip (expanded). The rows
// move together as one unit, so either row having content expands both.

enum KeyboardChrome {
    static func isComposing(romanHint: String, state: IosRenderState) -> Bool {
        !romanHint.isEmpty || !state.candidates.isEmpty
    }
}
