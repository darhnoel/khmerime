import UIKit

// KeyStyle
// ========
// Shared visual styling for all key buttons across every keyboard layer.
//
// Android equivalent
// ------------------
// Define a set of drawable resources in res/drawable/ and a Kotlin helper
// object that applies them. The three variants (letter, symbol, special)
// correspond to the same three button types on Android:
//
//   object KeyStyle {
//       fun applyLetter(btn: Button) { … }   // white background, 17sp
//       fun applySymbol(btn: Button) { … }   // white background, 17sp
//       fun applySpecial(btn: Button) { … }  // grey background, 15sp medium
//   }
//
// Visual guide:
//   Letter / Symbol keys:  white bg, 5pt radius, 1pt bottom shadow
//   Special keys (⌫ 💡 123 space ⏎ …): systemGray3 bg, same radius + shadow

enum KeyStyle {

    // MARK: - Public

    static func applyLetter(_ btn: UIButton) {
        apply(btn, white: true)
        btn.titleLabel?.font = .systemFont(ofSize: 17)
        btn.setTitleColor(.black, for: .normal)
    }

    static func applySymbol(_ btn: UIButton) {
        apply(btn, white: true)
        btn.titleLabel?.font = .systemFont(ofSize: 17)
        btn.setTitleColor(.black, for: .normal)
    }

    static func applySpecial(_ btn: UIButton) {
        apply(btn, white: false)
        btn.titleLabel?.font = .systemFont(ofSize: 15, weight: .medium)
        btn.setTitleColor(.black, for: .normal)
    }

    // MARK: - Private

    private static func apply(_ btn: UIButton, white: Bool) {
        btn.backgroundColor = white ? .white : UIColor.systemGray3
        btn.layer.cornerRadius = 5
        btn.layer.shadowColor = UIColor(white: 0, alpha: 1).cgColor
        btn.layer.shadowOpacity = 0.25
        btn.layer.shadowOffset = CGSize(width: 0, height: 1)
        btn.layer.shadowRadius = 0
        btn.layer.masksToBounds = false
    }
}
