import XCTest
import UIKit
@testable import KhmerIMEKeyboard

// CandidateSurfaceViewTests
// =========================
// The candidate-row slot hosts BOTH the Phrase Wheel (composition) and the
// word-level CandidateRowView (CharPick). ADR-0014 makes the wheel the default;
// the candidate row returns for CharPick character candidates.

final class CandidateSurfaceViewTests: XCTestCase {

    func test_compositionShowsWheelAndHidesCandidateRow() {
        let surface = CandidateSurfaceView()

        surface.render(makeState(phrases: ["ខ្ញុំ", "ញ៉ម"], candidates: ["ក"]), presentation: .composition)

        XCTAssertFalse(surface.wheel.isHidden, "the wheel is the default composition surface")
        XCTAssertTrue(surface.candidateRow.isHidden, "the word candidate row is hidden during composition")
    }

    func test_charPickShowsCandidateRowAndHidesWheel() {
        let surface = CandidateSurfaceView()

        surface.render(makeState(phrases: ["ខ្ញុំ"], candidates: ["ក", "្ក", "ខ"]), presentation: .charPick)

        XCTAssertTrue(surface.wheel.isHidden, "the wheel is hidden in CharPick")
        XCTAssertFalse(surface.candidateRow.isHidden, "CharPick character candidates use the candidate row")
    }

    private func makeState(phrases: [String], candidates: [String]) -> IosRenderState {
        IosRenderState(
            candidates: candidates, selectedIndex: nil, preedit: "", segments: [],
            focusedSegmentIndex: nil, commitText: nil, segmentEditActive: false, segmentEditIndex: nil,
            phraseCandidates: phrases.map { IosPhraseCandidate(text: $0, segments: []) }
        )
    }
}
