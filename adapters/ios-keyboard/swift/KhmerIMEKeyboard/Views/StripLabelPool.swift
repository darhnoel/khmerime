import UIKit

// Khmer glyphs can extend beyond the system font's nominal line box, especially
// Coeng Forms and below-base vowels. A plain UILabel sizes itself to that line
// box and clips those marks even when its parent row has spare height.
private final class KhmerGlyphLabel: UILabel {
    private static let verticalGlyphClearance: CGFloat = 12
    private static let haptic = UIImpactFeedbackGenerator(style: .light)
    var quickAccessFeedbackEnabled = false

    override var intrinsicContentSize: CGSize {
        var size = super.intrinsicContentSize
        if size.height != UIView.noIntrinsicMetric {
            size.height += Self.verticalGlyphClearance
        }
        return size
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        if quickAccessFeedbackEnabled {
            UIDevice.current.playInputClick()
            Self.haptic.impactOccurred(intensity: 0.5)
            Self.haptic.prepare()
            UIView.animate(withDuration: 0.08) {
                self.transform = CGAffineTransform(scaleX: 0.92, y: 0.92)
            }
        }
        super.touchesBegan(touches, with: event)
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        releaseQuickAccessPress()
        super.touchesEnded(touches, with: event)
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        releaseQuickAccessPress()
        super.touchesCancelled(touches, with: event)
    }

    private func releaseQuickAccessPress() {
        guard quickAccessFeedbackEnabled else { return }
        UIView.animate(withDuration: 0.22) { self.transform = .identity }
    }
}

// StripLabelPool
// ==============
// Manages a reusable pool of UILabels inside a UIStackView.
// Instead of removeFromSuperview + addArrangedSubview on every render,
// the pool grows on demand and hides/shows labels in place.
// Eliminates the UIStackView layout pass cost per keystroke (Fix B).

final class StripLabelPool {

    private(set) var labels: [UILabel] = []

    // Ensures exactly `count` visible labels are arranged in `stackView`.
    // Grows the pool when needed (adds new labels to stackView once).
    // Hides excess labels rather than removing them.
    // Returns the visible slice in order.
    @discardableResult
    func sync(count: Int, in stackView: UIStackView) -> [UILabel] {
        while labels.count < count {
            let label = KhmerGlyphLabel()
            label.isUserInteractionEnabled = true
            stackView.addArrangedSubview(label)
            labels.append(label)
        }
        for (i, label) in labels.enumerated() {
            label.isHidden = i >= count
        }
        return Array(labels.prefix(count))
    }

    func setQuickAccessFeedbackEnabled(_ enabled: Bool, for visibleLabels: [UILabel]) {
        labels.forEach {
            guard let label = $0 as? KhmerGlyphLabel else { return }
            label.quickAccessFeedbackEnabled = enabled && visibleLabels.contains { $0 === label }
            if !label.quickAccessFeedbackEnabled { label.transform = .identity }
        }
    }
}
