import XCTest
import UIKit
@testable import KhmerIMEKeyboard

final class StripViewTests: XCTestCase {

    // MARK: - segmentIndex(at:labelFrames:)

    func test_segmentIndex_pointInsideALabelFrame_returnsThatIndex() {
        let frames = [
            CGRect(x: 0, y: 0, width: 40, height: 30),
            CGRect(x: 50, y: 0, width: 40, height: 30),
        ]

        let idx = StripView.segmentIndex(at: CGPoint(x: 60, y: 10), labelFrames: frames)

        XCTAssertEqual(idx, 1, "point inside the second frame must resolve to index 1")
    }

    func test_segmentIndex_pointOutsideAllFrames_returnsNil() {
        let frames = [CGRect(x: 0, y: 0, width: 40, height: 30)]

        let idx = StripView.segmentIndex(at: CGPoint(x: 200, y: 200), labelFrames: frames)

        XCTAssertNil(idx, "a point that misses every label must mean 'tapped empty row space'")
    }

    func test_segmentIndex_noLabels_returnsNil() {
        let idx = StripView.segmentIndex(at: CGPoint(x: 10, y: 10), labelFrames: [])

        XCTAssertNil(idx, "no segments visible — any tap must fall through to the row handler")
    }
}
