import XCTest
import UIKit
@testable import KhmerIMEKeyboard

// PhraseWheelViewTests
// ====================
// The Phrase Wheel (ADR-0014) renders the ranked whole-phrase hypotheses as a
// horizontal, center-snapped carousel. One card per Phrase Candidate, in rank
// order; the centered card is the selection.

final class PhraseWheelViewTests: XCTestCase {

    func test_render_showsOneCardPerPhraseCandidateInOrder() {
        let wheel = PhraseWheelView()

        wheel.render(makeState(phrases: ["ខ្ញុំទៅសាលា", "ខ្ញុំទៅសាលារៀន", "khnhomtov"]))

        XCTAssertEqual(visibleLabelTexts(in: wheel), ["ខ្ញុំទៅសាលា", "ខ្ញុំទៅសាលារៀន", "khnhomtov"],
            "the wheel shows one card per Phrase Candidate, in rank order, raw roman last")
    }

    func test_clear_removesAllCards() {
        let wheel = PhraseWheelView()
        wheel.render(makeState(phrases: ["ខ្ញុំ", "ញ៉ម"]))

        wheel.clear()

        XCTAssertEqual(visibleLabelTexts(in: wheel), [], "clearing empties the wheel")
    }

    // MARK: - Helpers

    private func makeState(phrases: [String]) -> IosRenderState {
        IosRenderState(
            candidates: [],
            selectedIndex: nil,
            preedit: "",
            segments: [],
            focusedSegmentIndex: nil,
            commitText: nil,
            segmentEditActive: false,
            segmentEditIndex: nil,
            phraseCandidates: phrases.map { IosPhraseCandidate(text: $0, segments: []) }
        )
    }

    private func visibleLabelTexts(in view: UIView) -> [String?] {
        var result: [String?] = []
        for sub in view.subviews {
            if let label = sub as? UILabel, !label.isHidden { result.append(label.text) }
            result += visibleLabelTexts(in: sub)
        }
        return result
    }
}
