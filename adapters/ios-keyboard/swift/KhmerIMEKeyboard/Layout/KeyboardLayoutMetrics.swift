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
            baseKeyboardHeight = 304
            specialKeyWidth = 42
            returnKeyWidth = 82
            wideSpecialKeyWidth = 48
        case .pad:
            baseKeyboardHeight = 364
            specialKeyWidth = 56
            returnKeyWidth = 112
            wideSpecialKeyWidth = 72
        }
        stripHeight = 44
        candidateRowHeight = 44
        rowSpacing = 8
        keyHorizontalInset = 3
        keyTopInset = 8
        keyBottomInset = 4
    }
}
