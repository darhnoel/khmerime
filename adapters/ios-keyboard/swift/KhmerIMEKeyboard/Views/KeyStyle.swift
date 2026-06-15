import UIKit

final class GlassKeyButton: UIButton {
    var isGlassActive = false {
        didSet { updateGlassAppearance() }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        updateGlassAppearance()
    }

    override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        updateGlassAppearance()
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
