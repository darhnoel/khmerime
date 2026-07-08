import XCTest
@testable import KhmerIMEKeyboard

// PhraseWheelLayoutTests
// ======================
// The Phrase Wheel (ADR-0014) is a center-snapped horizontal carousel: the card
// nearest the view's horizontal center is the selected one. This is the pure
// snap math — which card a scroll offset lands on, and the offset that centers a
// given card — with no UIKit state.

final class PhraseWheelLayoutTests: XCTestCase {

    func test_nearestCardIndex_picksTheCardWhoseCenterIsClosest() {
        let centers: [CGFloat] = [50, 150, 260]
        XCTAssertEqual(PhraseWheelLayout.nearestCardIndex(toCenterX: 140, cardCenters: centers), 1,
            "140 is nearest the second card's center (150)")
        XCTAssertEqual(PhraseWheelLayout.nearestCardIndex(toCenterX: 40, cardCenters: centers), 0)
        XCTAssertEqual(PhraseWheelLayout.nearestCardIndex(toCenterX: 10_000, cardCenters: centers), 2,
            "scrolling past the end snaps to the last card")
    }

    func test_nearestCardIndex_emptyReturnsNil() {
        XCTAssertNil(PhraseWheelLayout.nearestCardIndex(toCenterX: 0, cardCenters: []))
    }

    func test_centerOffset_placesTheCardCenterAtTheViewMiddle() throws {
        // card center 260, view 180 wide → 260 - 90 = 170 puts it dead center.
        let offset = try XCTUnwrap(
            PhraseWheelLayout.centerOffset(forCardIndex: 2, cardCenters: [50, 150, 260], viewWidth: 180))
        XCTAssertEqual(offset, 170, accuracy: 0.001)
    }

    func test_centerOffset_outOfRangeReturnsNil() {
        XCTAssertNil(PhraseWheelLayout.centerOffset(forCardIndex: 9, cardCenters: [50], viewWidth: 100))
    }
}
