import UIKit

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
        }

        if let backspace = view as? BackspaceButton {
            backspace.onTap = nil
            backspace.onHoldFire = nil
            backspace.onHoldEnd = nil
        }

        if let globe = view as? GlobeKeyButton {
            globe.onShortTap = nil
            globe.onLongPress = nil
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
