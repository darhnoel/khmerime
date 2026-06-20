import UIKit

// Silk Veil brand tokens — KhmerIME's shared visual identity (see ADR-0010,
// ADR-0011). Colors are duplicated as native constants on each platform; the
// logo and app icon ship as exported PNG assets, not as code.
enum Brand {
    static let ink = UIColor(hex: 0x14101B)            // deep-ink / charcoal-plum base
    static let amber = UIColor(hex: 0xE98A4E)          // ember-amber primary action
    static let ivory = UIColor(hex: 0xF4ECE2)          // warm ivory text
    static let teal = UIColor(hex: 0x38C6C0)           // peacock-teal accent
    static let ivoryDim = UIColor(hex: 0xF4ECE2, alpha: 0.62)
}

extension UIColor {
    convenience init(hex: Int, alpha: CGFloat = 1.0) {
        self.init(
            red: CGFloat((hex >> 16) & 0xFF) / 255.0,
            green: CGFloat((hex >> 8) & 0xFF) / 255.0,
            blue: CGFloat(hex & 0xFF) / 255.0,
            alpha: alpha
        )
    }
}
