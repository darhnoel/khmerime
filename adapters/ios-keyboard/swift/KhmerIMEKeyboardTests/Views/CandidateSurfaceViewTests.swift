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

    func test_compositionWithNoAlternativesHidesTheWheel() {
        // ADR-0015: one hypothesis → the strip already shows it → the wheel hides so the
        // strip stands alone (no redundant single card).
        let surface = CandidateSurfaceView()

        surface.render(makeState(phrases: ["ខ្ញុំ"], candidates: ["ក"]), presentation: .composition)

        XCTAssertTrue(surface.wheel.isHidden, "with no alternatives the wheel is hidden")
    }

    func test_charPickShowsCandidateRowAndHidesWheel() {
        let surface = CandidateSurfaceView()

        surface.render(makeState(phrases: ["ខ្ញុំ"], candidates: ["ក", "្ក", "ខ"]), presentation: .charPick)

        XCTAssertTrue(surface.wheel.isHidden, "the wheel is hidden in CharPick")
        XCTAssertFalse(surface.candidateRow.isHidden, "CharPick character candidates use the candidate row")
    }

    func test_segmentEditActiveShowsWordCandidatesNotWheel() {
        // ADR-0014 Level 2: double-touch a card → edit one word. While editing, the
        // focused segment's word candidates show (in the candidate row), not the wheel.
        let surface = CandidateSurfaceView()

        surface.render(
            makeState(phrases: ["ខ្ញុំទៅ"], candidates: ["ខ្ញុំ", "ញ៉ម"], segmentEditActive: true),
            presentation: .composition)

        XCTAssertTrue(surface.wheel.isHidden, "the wheel is hidden while editing a word (Level 2)")
        XCTAssertFalse(surface.candidateRow.isHidden, "the focused segment's word candidates show during edit")
    }

    private func makeState(phrases: [String], candidates: [String], segmentEditActive: Bool = false) -> IosRenderState {
        IosRenderState(
            candidates: candidates, selectedIndex: nil, preedit: "", segments: [],
            focusedSegmentIndex: nil, commitText: nil, segmentEditActive: segmentEditActive, segmentEditIndex: nil,
            phraseCandidates: phrases.map { IosPhraseCandidate(text: $0, segments: []) },
            selectedPhraseIndex: 0
        )
    }
}
