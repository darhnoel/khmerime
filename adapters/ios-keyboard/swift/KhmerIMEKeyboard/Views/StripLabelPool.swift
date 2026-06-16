import UIKit

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
            let label = UILabel()
            label.isUserInteractionEnabled = true
            stackView.addArrangedSubview(label)
            labels.append(label)
        }
        for (i, label) in labels.enumerated() {
            label.isHidden = i >= count
        }
        return Array(labels.prefix(count))
    }
}
