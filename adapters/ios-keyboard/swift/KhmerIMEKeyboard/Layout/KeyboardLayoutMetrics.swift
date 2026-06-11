import UIKit

struct KeyboardLayoutMetrics {
    enum Device {
        case phone
        case pad
    }

    let baseKeyboardHeight: CGFloat
    let stripHeight: CGFloat
    let specialKeyWidth: CGFloat
    let returnKeyWidth: CGFloat
    let wideSpecialKeyWidth: CGFloat
    let rowSpacing: CGFloat
    let keyHorizontalInset: CGFloat
    let keyTopInset: CGFloat
    let keyBottomInset: CGFloat
    let panelChipHeight: CGFloat
    let panelCandidateHeight: CGFloat
    let panelBottomRowTopSpacing: CGFloat
    let panelBottomRowHeight: CGFloat

    init(device: Device) {
        switch device {
        case .phone:
            baseKeyboardHeight = 260
            specialKeyWidth = 42
            returnKeyWidth = 82
            wideSpecialKeyWidth = 48
        case .pad:
            baseKeyboardHeight = 320
            specialKeyWidth = 56
            returnKeyWidth = 112
            wideSpecialKeyWidth = 72
        }
        stripHeight = 44
        rowSpacing = 8
        keyHorizontalInset = 3
        keyTopInset = 8
        keyBottomInset = 4
        panelChipHeight = 44
        panelCandidateHeight = 120
        panelBottomRowTopSpacing = 8
        panelBottomRowHeight = 44
    }
}
