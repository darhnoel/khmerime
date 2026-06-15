import UIKit

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

    static func applySpecial(_ btn: UIButton, isIPad: Bool = false) {
        applyGlass(btn, isIPad: isIPad)
        btn.titleLabel?.font = .systemFont(ofSize: isIPad ? 17 : 15, weight: .medium)
        btn.setTitleColor(.label, for: .normal)
    }

    private static func applyGlass(_ btn: UIButton, isIPad: Bool) {
        btn.backgroundColor = UIColor { traits in
            GlassColorSpec.backgroundColor(isDark: traits.userInterfaceStyle == .dark)
        }
        btn.layer.cornerRadius = isIPad ? 8 : 5
        btn.layer.borderWidth = 1
        btn.layer.borderColor = GlassColorSpec.borderColor(isDark: false).cgColor
        btn.layer.shadowOpacity = 0
        btn.layer.masksToBounds = false
    }
}
