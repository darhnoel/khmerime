import Foundation

// KeyboardSession
// ===============
// Wraps the UniFFI-generated KhmerImeSession and documents the render-loop
// contract that every platform adapter must follow.
//
// Android equivalent
// ------------------
// Create an identical wrapper in Kotlin around the UniFFI KhmerImeSession.
// The method names, initialisation parameters, and render-loop invariants
// are the same. Only the view layer (UIKit vs Android Views) differs.
//
//   class KeyboardSession(context: Context) {
//       private val inner = KhmerImeSession()
//       fun sendCharacter(ch: String): IosRenderState = inner.processCharacter(ch)
//       fun sendBackspace(): IosRenderState = inner.processBackspace()
//       // … same pattern for all methods
//   }
//
// Render-loop invariant
// ---------------------
// Every key tap — letter, digit, symbol, backspace, space, return — MUST go
// through the session. Never insert text into the host text field without
// consulting the session first.
//
// The session owns romanisation + segmentation + candidate ranking.
// The UI only reads IosRenderState and updates views accordingly.
//
// Roman-buffer pattern
// --------------------
// iOS (and Android) speculatively insert the roman char into the host text
// field as the user types. On ⏎ the roman chars are bulk-deleted and replaced
// with the committed Khmer text. `romanBuffer` in KeyboardViewController
// (or its Android equivalent) tracks exactly what has been inserted so the
// controller knows how many deleteBackward() / deleteSurroundingText() calls
// to make.
//
// Session lifecycle
// -----------------
// Create once in viewDidLoad / onCreateInputView.
// Call focusIn() when the keyboard becomes visible.
// Call focusOut() when it disappears.

final class KeyboardSession {

    private let inner: KhmerImeSession

    init() {
        inner = KhmerImeSession()
    }

    // MARK: - Lifecycle

    func focusIn()  -> IosRenderState { inner.focusIn() }
    func focusOut() -> IosRenderState { inner.focusOut() }

    // MARK: - Key routing

    func sendCharacter(_ ch: String) -> IosRenderState { inner.processCharacter(ch: ch) }
    func sendBackspace()             -> IosRenderState { inner.processBackspace() }
    func sendSpace()                 -> IosRenderState { inner.processSpace() }
    func sendReturn()                -> IosRenderState { inner.processEnter() }

    // Toggles Segment Edit Mode on the focused segment (Tab key / 0xFF09).
    // First call: enters edit mode — next printable replaces the segment's roman input.
    // Second call: exits edit mode and pins the segment.
    func sendTab() -> IosRenderState { inner.processTab() }

    // Moves segment focus one step left / right within a segmented composition.
    func sendLeft()  -> IosRenderState { inner.processLeft() }
    func sendRight() -> IosRenderState { inner.processRight() }

    // Selects the candidate at `index` (0-based) for the focused segment.
    // The session maps: index 0 → digit key '1', index 1 → '2', … index 8 → '9'.
    // Indices ≥ 9 are not reachable via digit keys; the caller must guard.
    func selectCandidate(at index: Int) -> IosRenderState {
        guard index < 9 else { return inner.focusIn() }
        return inner.processDigit(n: UInt8(index + 1))
    }

    // MARK: - CharPick mode

    // Enters CharPick mode. Clears any active composition.
    // Returns a clean render state with empty candidates (no roman letter typed yet).
    func enterCharPick() -> IosRenderState { inner.enterCharPick() }

    // Exits CharPick mode and returns to Roman mode.
    func exitCharPick() -> IosRenderState { inner.exitCharPick() }
}
