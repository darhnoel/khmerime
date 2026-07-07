import CoreGraphics

// PhraseWheelLayout
// =================
// Pure snap math for the horizontal Phrase Wheel (ADR-0014). The wheel is a
// center-snapped carousel: whichever card's center is nearest the view's
// horizontal center is the selected Phrase Candidate. These helpers convert a
// scroll offset to the centered card index and back, with no UIKit state, so the
// snapping and selection reporting are unit-testable.

enum PhraseWheelLayout {

    /// The index of the card whose center is closest to `centerX` (a point in the
    /// scroll view's content coordinates, usually `contentOffset.x + viewWidth/2`).
    /// `nil` when there are no cards.
    static func nearestCardIndex(toCenterX centerX: CGFloat, cardCenters: [CGFloat]) -> Int? {
        guard !cardCenters.isEmpty else { return nil }
        var best = 0
        var bestDistance = CGFloat.greatestFiniteMagnitude
        for (index, center) in cardCenters.enumerated() {
            let distance = abs(center - centerX)
            if distance < bestDistance {
                bestDistance = distance
                best = index
            }
        }
        return best
    }

    /// The `contentOffset.x` that places card `index`'s center at the view's
    /// horizontal center. `nil` when `index` is out of range. Clamping to the
    /// scrollable range is the caller's responsibility.
    static func centerOffset(forCardIndex index: Int, cardCenters: [CGFloat], viewWidth: CGFloat) -> CGFloat? {
        guard cardCenters.indices.contains(index) else { return nil }
        return cardCenters[index] - viewWidth / 2
    }
}
