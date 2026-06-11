import XCTest
@testable import KhmerIMEKeyboard

final class KeyboardLayoutMetricsTests: XCTestCase {

    func test_metricsPreserveCurrentPhoneAndPadSizingPolicy() {
        let phone = KeyboardLayoutMetrics(device: .phone)
        let pad = KeyboardLayoutMetrics(device: .pad)

        XCTAssertEqual(phone.baseKeyboardHeight, 260)
        XCTAssertEqual(phone.stripHeight, 44)
        XCTAssertEqual(phone.specialKeyWidth, 42)
        XCTAssertEqual(phone.returnKeyWidth, 82)
        XCTAssertEqual(phone.wideSpecialKeyWidth, 48)
        XCTAssertEqual(phone.rowSpacing, 8)
        XCTAssertEqual(phone.keyHorizontalInset, 3)
        XCTAssertEqual(phone.keyTopInset, 8)
        XCTAssertEqual(phone.keyBottomInset, 4)
        XCTAssertEqual(phone.panelChipHeight, 44)
        XCTAssertEqual(phone.panelCandidateHeight, 120)
        XCTAssertEqual(phone.panelBottomRowTopSpacing, 8)
        XCTAssertEqual(phone.panelBottomRowHeight, 44)

        XCTAssertEqual(pad.baseKeyboardHeight, 320)
        XCTAssertEqual(pad.stripHeight, 44)
        XCTAssertEqual(pad.specialKeyWidth, 56)
        XCTAssertEqual(pad.returnKeyWidth, 112)
        XCTAssertEqual(pad.wideSpecialKeyWidth, 72)
        XCTAssertEqual(pad.rowSpacing, 8)
        XCTAssertEqual(pad.keyHorizontalInset, 3)
        XCTAssertEqual(pad.keyTopInset, 8)
        XCTAssertEqual(pad.keyBottomInset, 4)
        XCTAssertEqual(pad.panelChipHeight, 44)
        XCTAssertEqual(pad.panelCandidateHeight, 120)
        XCTAssertEqual(pad.panelBottomRowTopSpacing, 8)
        XCTAssertEqual(pad.panelBottomRowHeight, 44)
    }
}
