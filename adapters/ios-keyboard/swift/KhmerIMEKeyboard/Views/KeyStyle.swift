import UIKit

final class GlassKeyButton: UIButton {
    var isGlassActive = false {
        didSet { updateGlassAppearance() }
    }

    private var pressAnimator: GlassKeyPressAnimator?

    // Inject a synchronous runner for testing before any touch event fires.
    func configureForTesting(runner: @escaping AnimatorRunner) {
        pressAnimator = GlassKeyPressAnimator(onUpdate: applySquish(_:), runner: runner)
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesBegan(touches, with: event)
        ensurePressAnimator().press()
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesEnded(touches, with: event)
        ensurePressAnimator().release()
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesCancelled(touches, with: event)
        ensurePressAnimator().release()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        updateGlassAppearance()
    }

    override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        updateGlassAppearance()
    }

    private func ensurePressAnimator() -> GlassKeyPressAnimator {
        if let existing = pressAnimator { return existing }
        let animator = GlassKeyPressAnimator(onUpdate: applySquish(_:))
        pressAnimator = animator
        return animator
    }

    private func applySquish(_ squish: CGFloat) {
        let scale = 1 - squish * 0.08
        transform = CGAffineTransform(scaleX: scale, y: scale)
    }

    private func updateGlassAppearance() {
        KeyStyle.updateGlassAppearance(self, isActive: isGlassActive)
    }
}

enum KeyStyle {

    static func applyLetter(_ btn: UIButton, isIPad: Bool = false) {
        applyGlass(btn, isIPad: isIPad)
        btn.titleLabel?.font = .systemFont(ofSize: isIPad ? 20 : 17)
        btn.setTitleColor(.label, for: .normal)
    }

    static func applySymbol(_ btn: UIButton, isIPad: Bool = false) {
        applyGlass(btn, isIPad: isIPad)
        btn.titleLabel?.font = .systemFont(ofSize: isIPad ? 20 : 17)
        btn.setTitleColor(.label, for: .normal)
    }

    static func applySpecial(_ btn: UIButton, isIPad: Bool = false, isActive: Bool = false) {
        if let glassButton = btn as? GlassKeyButton {
            glassButton.isGlassActive = isActive
        }
        applyGlass(btn, isIPad: isIPad)
        btn.titleLabel?.font = .systemFont(ofSize: isIPad ? 17 : 15, weight: .medium)
    }

    private static func applyGlass(_ btn: UIButton, isIPad _: Bool) {
        updateGlassAppearance(btn, isActive: (btn as? GlassKeyButton)?.isGlassActive ?? false)
        btn.layer.borderWidth = 1
        btn.layer.shadowOpacity = 0
        btn.layer.masksToBounds = false
    }

    fileprivate static func updateGlassAppearance(_ btn: UIButton, isActive: Bool) {
        let isDark = btn.traitCollection.userInterfaceStyle == .dark
        btn.backgroundColor = isActive
            ? GlassColorSpec.toggleActiveBackground(isDark: isDark)
            : GlassColorSpec.backgroundColor(isDark: isDark)
        btn.setTitleColor(isActive ? GlassColorSpec.toggleActiveTextColor() : .label, for: .normal)
        btn.layer.cornerRadius = GlassColorSpec.keyCornerRadius(height: btn.bounds.height)
        btn.layer.borderColor = GlassColorSpec.borderColor(isDark: isDark).cgColor
    }
}
