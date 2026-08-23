import UIKit

// Tag identifying the next-keyboard globe button, shared by the layer factory,
// the controller (visibility + wiring), and teardown. A top-level constant so
// it is reachable from the test target too (which compiles teardown without the
// controller).
let globeKeyViewTag = 999

enum KeyboardResourceTeardown {
    static func releaseInteractions(in view: UIView) {
        releaseInteraction(on: view)
        view.subviews.forEach { releaseInteractions(in: $0) }
    }

    private static func releaseInteraction(on view: UIView) {
        view.gestureRecognizers?.forEach { view.removeGestureRecognizer($0) }

        if let control = view as? UIControl {
            control.removeTarget(nil, action: nil, for: .allEvents)
        }

        if let key = view as? GlassKeyButton {
            key.onPress = nil
            key.onPreviewChanged = nil
        }

        if let backspace = view as? BackspaceButton {
            backspace.onTap = nil
            backspace.onHoldFire = nil
            backspace.onHoldEnd = nil
        }

        // The globe is a plain button wired via addTarget for .allTouchEvents
        // (next-keyboard handling); drop that target so nothing retains the
        // controller after teardown.
        if let globe = view as? UIButton, globe.tag == globeKeyViewTag {
            globe.removeTarget(nil, action: nil, for: .allTouchEvents)
        }

        if let strip = view as? StripView {
            strip.onKhmerRowTapped = nil
            strip.onKhmerRowLongPressed = nil
            strip.onSegmentFocused = nil
        }

        if let candidateRow = view as? CandidateRowView {
            candidateRow.onCandidateSelected = nil
        }

        if let wheel = view as? PhraseWheelView {
            wheel.onPhraseSelected = nil
        }
    }
}
