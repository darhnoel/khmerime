import XCTest
@testable import KhmerIMEKeyboard

// CandidateRowLayoutTests
// =======================
// The candidate row centers its chips while they all fit, so a sparse set looks
// balanced; once the chips overflow the visible width it falls back to the normal
// edge inset (left-aligned start + horizontal scroll).

final class CandidateRowLayoutTests: XCTestCase {

    func test_centeringInset_narrowContent_centersWithSymmetricInset() {
        // 100pt of chips in a 300pt row → (300 - 100) / 2 = 100pt each side.
        let inset = CandidateRowLayout.centeringInset(contentWidth: 100, availableWidth: 300, edgeInset: 8)
        XCTAssertEqual(inset, 100, accuracy: 0.001,
            "content narrower than the row must be centered: inset = (available - content) / 2")
    }

    func test_centeringInset_overflowingContent_usesEdgeInset() {
        // 500pt of chips in a 300pt row → overflow, no centering, normal edge inset.
        let inset = CandidateRowLayout.centeringInset(contentWidth: 500, availableWidth: 300, edgeInset: 8)
        XCTAssertEqual(inset, 8, accuracy: 0.001,
            "content wider than the row must left-align and scroll: inset = edgeInset")
    }

    func test_centeringInset_exactlyFits_usesEdgeInset() {
        let inset = CandidateRowLayout.centeringInset(contentWidth: 300, availableWidth: 300, edgeInset: 8)
        XCTAssertEqual(inset, 8, accuracy: 0.001,
            "content exactly as wide as the row leaves no room to center: inset = edgeInset")
    }
}
