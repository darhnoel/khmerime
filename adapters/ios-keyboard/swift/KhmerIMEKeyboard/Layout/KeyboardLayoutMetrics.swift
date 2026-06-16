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
        candidateRowHeight = 44
        rowSpacing = 8
        keyHorizontalInset = 3
        keyTopInset = 8
        keyBottomInset = 4
        panelChipHeight = 44
        panelBottomRowTopSpacing = 8

        let keyAreaHeight = baseKeyboardHeight - stripHeight
        let standardRowHeight = (
            keyAreaHeight
            - keyTopInset
            - keyBottomInset
            - (rowSpacing * 3)
        ) / 4
        let standardBottomRowTop = keyTopInset + (standardRowHeight + rowSpacing) * 3

        panelBottomRowHeight = standardRowHeight
        panelCandidateHeight = (
            standardBottomRowTop
            - 4
            - panelChipHeight
            - 0.5
            - 0.5
            - panelBottomRowTopSpacing
        )
    }
}
