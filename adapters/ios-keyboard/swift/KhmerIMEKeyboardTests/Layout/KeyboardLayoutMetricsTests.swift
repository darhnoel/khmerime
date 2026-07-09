import XCTest
@testable import KhmerIMEKeyboard

final class KeyboardLayoutMetricsTests: XCTestCase {

    func test_candidateRowHeightIsReservedForPhoneAndPad() {
        XCTAssertEqual(KeyboardLayoutMetrics(device: .phone).candidateRowHeight, 44)
        XCTAssertEqual(KeyboardLayoutMetrics(device: .pad).candidateRowHeight, 44)
    }

    func test_metricsPreserveCurrentPhoneAndPadSizingPolicy() {
        let phone = KeyboardLayoutMetrics(device: .phone)
        let pad = KeyboardLayoutMetrics(device: .pad)

        XCTAssertEqual(phone.baseKeyboardHeight, 310)
        XCTAssertEqual(phone.stripHeight, 50)
        XCTAssertEqual(phone.specialKeyWidth, 42)
        XCTAssertEqual(phone.returnKeyWidth, 82)
        XCTAssertEqual(phone.wideSpecialKeyWidth, 48)
        XCTAssertEqual(phone.rowSpacing, 8)
        XCTAssertEqual(phone.keyHorizontalInset, 3)
        XCTAssertEqual(phone.keyTopInset, 8)
        XCTAssertEqual(phone.keyBottomInset, 4)

        XCTAssertEqual(pad.baseKeyboardHeight, 370)
        XCTAssertEqual(pad.stripHeight, 50)
        XCTAssertEqual(pad.specialKeyWidth, 56)
        XCTAssertEqual(pad.returnKeyWidth, 112)
        XCTAssertEqual(pad.wideSpecialKeyWidth, 72)
        XCTAssertEqual(pad.rowSpacing, 8)
        XCTAssertEqual(pad.keyHorizontalInset, 3)
        XCTAssertEqual(pad.keyTopInset, 8)
        XCTAssertEqual(pad.keyBottomInset, 4)
    }

    func test_idleKeyboardHeightDropsBothChromeRows() {
        // When idle (no composition) the strip + candidate row collapse to zero, so
        // the keyboard's total height is the full height minus both reserved rows.
        for metrics in [KeyboardLayoutMetrics(device: .phone), KeyboardLayoutMetrics(device: .pad)] {
            XCTAssertEqual(
                metrics.idleKeyboardHeight,
                metrics.baseKeyboardHeight - metrics.stripHeight - metrics.candidateRowHeight,
                "idle height must drop exactly the strip + candidate row (88pt), keeping the key area unchanged"
            )
        }
    }

    func test_keyRowHeightStaysAboveTouchTargetFloorAfterCandidateRowInsertion() {
        for metrics in [KeyboardLayoutMetrics(device: .phone), KeyboardLayoutMetrics(device: .pad)] {
            let effectiveKeyAreaHeight = metrics.baseKeyboardHeight - metrics.stripHeight - metrics.candidateRowHeight
            let standardRowHeight = (
                effectiveKeyAreaHeight
                - metrics.keyTopInset
                - metrics.keyBottomInset
                - (metrics.rowSpacing * 3)
            ) / 4

            XCTAssertGreaterThanOrEqual(standardRowHeight, 44)
        }
    }
}
