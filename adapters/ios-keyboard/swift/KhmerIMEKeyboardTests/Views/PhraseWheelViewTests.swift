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

    func test_settlingWithACardCenteredReportsThatCardsIndex() throws {
        let wheel = PhraseWheelView()
        wheel.frame = CGRect(x: 0, y: 0, width: 220, height: 48)
        var reported: Int?
        wheel.onPhraseSelected = { reported = $0 }
        wheel.render(makeState(phrases: ["ខ្ញុំទៅសាលារៀន", "ខ្ញុំទៅសាលា", "ខ្ញុំទៅ", "khnhom"]))
        wheel.layoutIfNeeded()

        let offset = try XCTUnwrap(wheel.centerOffset(forCardIndex: 2))
        wheel.settleSelection(atContentOffsetX: offset)

        XCTAssertEqual(reported, 2, "settling with card 2 centered must report index 2 (→ selectPhrase)")
    }

    func test_settlingHighlightsOnlyTheCenteredCard() throws {
        let wheel = PhraseWheelView()
        wheel.frame = CGRect(x: 0, y: 0, width: 220, height: 48)
        wheel.render(makeState(phrases: ["ក", "ខ", "គ", "ឃ"]))
        wheel.layoutIfNeeded()

        let offset = try XCTUnwrap(wheel.centerOffset(forCardIndex: 1))
        wheel.settleSelection(atContentOffsetX: offset)

        XCTAssertEqual(visibleLabels(in: wheel).map { $0.textColor },
                       [.secondaryLabel, .label, .secondaryLabel, .secondaryLabel],
            "only the centered card is highlighted (.label); the rest dim to .secondaryLabel")
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
        visibleLabels(in: view).map { $0.text }
    }

    private func visibleLabels(in view: UIView) -> [UILabel] {
        var result: [UILabel] = []
        for sub in view.subviews {
            if let label = sub as? UILabel, !label.isHidden { result.append(label) }
            result += visibleLabels(in: sub)
        }
        return result
    }
}
