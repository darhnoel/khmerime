import XCTest
import UIKit
@testable import KhmerIMEKeyboard

// PhraseWheelViewTests
// ====================
// The Phrase Wheel (ADR-0015) renders the *alternative* whole-phrase hypotheses —
// the ones other than the top-ranked reading, which the strip shows. Tapping a card
// commits that phrase.

final class PhraseWheelViewTests: XCTestCase {

    func test_render_showsTheAlternativesNotTheTopHypothesis() {
        let wheel = PhraseWheelView()

        wheel.render(makeState(phrases: ["ខ្ញុំទៅសាលា", "ខ្ញុំទៅសាលារៀន", "ខ្ញុំទៅ"]))

        // The top hypothesis (ខ្ញុំទៅសាលា) is the strip's; the wheel shows the rest.
        XCTAssertEqual(visibleLabelTexts(in: wheel), ["ខ្ញុំទៅសាលារៀន", "ខ្ញុំទៅ"])
    }

    func test_singleHypothesis_hasNoAlternativesAndRendersEmpty() {
        let wheel = PhraseWheelView()

        wheel.render(makeState(phrases: ["ខ្ញុំ"]))

        XCTAssertEqual(visibleLabelTexts(in: wheel), [], "one hypothesis → nothing to choose → empty wheel")
        XCTAssertFalse(wheel.hasAlternatives)
    }

    func test_clear_removesAllCards() {
        let wheel = PhraseWheelView()
        wheel.render(makeState(phrases: ["ខ្ញុំ", "ញ៉ម", "ញំ"]))

        wheel.clear()

        XCTAssertEqual(visibleLabelTexts(in: wheel), [])
        XCTAssertFalse(wheel.hasAlternatives)
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
