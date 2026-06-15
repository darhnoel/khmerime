import UIKit

enum KeyboardHostLayout {
    static func heightConstant(metrics: KeyboardLayoutMetrics, safeAreaBottom: CGFloat) -> CGFloat {
        metrics.baseKeyboardHeight + safeAreaBottom
    }

    static func install(
        rootView: UIView,
        in hostView: UIView,
        metrics: KeyboardLayoutMetrics,
        safeAreaBottom: CGFloat
    ) -> NSLayoutConstraint {
        // Transparent so UIVisualEffectView on each key blurs the app content below.
        hostView.backgroundColor = .clear

        let heightConstraint = hostView.heightAnchor.constraint(
            equalToConstant: heightConstant(metrics: metrics, safeAreaBottom: safeAreaBottom)
        )
        heightConstraint.priority = UILayoutPriority(999)
        heightConstraint.isActive = true

        rootView.translatesAutoresizingMaskIntoConstraints = false
        hostView.addSubview(rootView)

        NSLayoutConstraint.activate([
            rootView.topAnchor.constraint(equalTo: hostView.topAnchor),
            rootView.leadingAnchor.constraint(equalTo: hostView.leadingAnchor),
            rootView.trailingAnchor.constraint(equalTo: hostView.trailingAnchor),
            rootView.bottomAnchor.constraint(equalTo: hostView.safeAreaLayoutGuide.bottomAnchor),
        ])

        return heightConstraint
    }
}
