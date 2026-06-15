import UIKit

enum GlassColorSpec {

    // Glass surface for individual keys.
    static func backgroundColor(isDark: Bool) -> UIColor {
        isDark
            ? UIColor(red: 90/255, green: 90/255, blue: 100/255, alpha: 210/255)
            : UIColor(red: 1, green: 1, blue: 1, alpha: 230/255)
    }

    // Background of the whole keyboard body (darker than the keys).
    static func keyboardBackground(isDark: Bool) -> UIColor {
        isDark
            ? UIColor(white: 0.15, alpha: 0.95)
            : UIColor(red: 209/255, green: 213/255, blue: 219/255, alpha: 1)
    }

    static func borderColor(isDark: Bool) -> UIColor {
        isDark
            ? UIColor(white: 1, alpha: 60/255)
            : UIColor(white: 0, alpha: 40/255)
    }

    static func toggleActiveBackground(isDark: Bool) -> UIColor {
        isDark
            ? UIColor(red: 240/255, green: 240/255, blue: 245/255, alpha: 245/255)
            : UIColor(red: 1, green: 1, blue: 1, alpha: 245/255)
    }

    static func toggleActiveTextColor() -> UIColor {
        UIColor(red: 20/255, green: 20/255, blue: 24/255, alpha: 1)
    }

    static func selectedCandidateBackground(isDark: Bool) -> UIColor {
        isDark
            ? UIColor(red: 110/255, green: 110/255, blue: 124/255, alpha: 235/255)
            : UIColor(red: 210/255, green: 215/255, blue: 225/255, alpha: 245/255)
    }

    static func candidateBorderWidth() -> CGFloat { 1.5 }

    static func blurRadius(scale: CGFloat) -> CGFloat { 12 * scale }
}
