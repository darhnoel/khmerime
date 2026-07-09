import UIKit

struct KeyboardLayoutMetrics {
    enum Device {
        case phone
        case pad
    }

    let baseKeyboardHeight: CGFloat
    let stripHeight: CGFloat
    let candidateRowHeight: CGFloat
    let specialKeyWidth: CGFloat
    let returnKeyWidth: CGFloat
    let wideSpecialKeyWidth: CGFloat
    let rowSpacing: CGFloat
    let keyHorizontalInset: CGFloat
    let keyTopInset: CGFloat
    let keyBottomInset: CGFloat

    init(device: Device) {
        switch device {
        case .phone:
            // +6 over the key area to fund the taller strip below (keeps keys the same size).
            baseKeyboardHeight = 310
            specialKeyWidth = 42
            returnKeyWidth = 82
            wideSpecialKeyWidth = 48
        case .pad:
            baseKeyboardHeight = 370
            specialKeyWidth = 56
            returnKeyWidth = 112
            wideSpecialKeyWidth = 72
        }
        // Taller than the candidate row so the Khmer row's stacked glyphs (coeng
        // subscripts + below-base vowels) aren't clipped at the bottom.
        stripHeight = 50
        candidateRowHeight = 44
        rowSpacing = 8
        keyHorizontalInset = 3
        keyTopInset = 8
        keyBottomInset = 4
    }

    // Height when idle: the strip + candidate row collapse to zero, leaving only
    // the key area. The keyboard expands to baseKeyboardHeight while composing.
    var idleKeyboardHeight: CGFloat {
        baseKeyboardHeight - stripHeight - candidateRowHeight
    }
}
