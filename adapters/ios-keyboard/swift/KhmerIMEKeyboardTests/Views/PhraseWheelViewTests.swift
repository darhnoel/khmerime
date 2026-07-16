import XCTest
import UIKit
@testable import KhmerIMEKeyboard

// PhraseWheelViewTests
// ====================
// The Phrase Wheel (ADR-0015) renders the *alternative* whole-phrase hypotheses —
// the ones other than the selected reading, which the strip shows. Tapping a card
// selects that phrase.

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

    func test_render_excludesSelectedHypothesisNotAlwaysTheTopHypothesis() {
        let wheel = PhraseWheelView()

        wheel.render(makeState(
            phrases: ["ខ្ញុំទៅសាលា", "ខ្ញុំទៅសាលារៀន", "ខ្ញុំទៅ"],
            selectedPhraseIndex: 1
        ))

        XCTAssertEqual(visibleLabelTexts(in: wheel), ["ខ្ញុំទៅសាលា", "ខ្ញុំទៅ"])
        XCTAssertEqual(visibleLabelTags(in: wheel), [0, 2])
    }

    func test_clear_removesAllCards() {
        let wheel = PhraseWheelView()
        wheel.render(makeState(phrases: ["ខ្ញុំ", "ញ៉ម", "ញំ"]))

        wheel.clear()

        XCTAssertEqual(visibleLabelTexts(in: wheel), [])
        XCTAssertFalse(wheel.hasAlternatives)
    }

    func test_unverifiedModelPhrase_coloursOnlyTheStarRed() {
        let wheel = PhraseWheelView()
        let state = IosRenderState(
            candidates: [], selectedIndex: nil, preedit: "", segments: [],
            focusedSegmentIndex: nil, commitText: nil, segmentEditActive: false, segmentEditIndex: nil,
            phraseCandidates: [
                IosPhraseCandidate(text: "គហបតី", segments: [], fromModel: false, lexiconVerified: true),
                IosPhraseCandidate(text: "គហិបតី", segments: [], fromModel: true, lexiconVerified: false),
            ],
            selectedPhraseIndex: 0
        )

        wheel.render(state)

        let label = visibleLabels(in: wheel).first
        XCTAssertEqual(label?.attributedText?.string, "✦ គហិបតី")
        XCTAssertEqual(label?.attributedText?.attribute(.foregroundColor, at: 0, effectiveRange: nil) as? UIColor,
            .systemRed, "the unverified model marker must be red")
        XCTAssertEqual(label?.attributedText?.attribute(.foregroundColor, at: 2, effectiveRange: nil) as? UIColor,
            .label, "the Khmer suggestion itself must keep the normal text colour")
    }

    // MARK: - Helpers

    private func makeState(phrases: [String], selectedPhraseIndex: UInt64 = 0) -> IosRenderState {
        IosRenderState(
            candidates: [],
            selectedIndex: nil,
            preedit: "",
            segments: [],
            focusedSegmentIndex: nil,
            commitText: nil,
            segmentEditActive: false,
            segmentEditIndex: nil,
            phraseCandidates: phrases.map {
                IosPhraseCandidate(text: $0, segments: [], fromModel: false, lexiconVerified: true)
            },
            selectedPhraseIndex: selectedPhraseIndex
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

    private func visibleLabels(in view: UIView) -> [UILabel] {
        var result: [UILabel] = []
        for sub in view.subviews {
            if let label = sub as? UILabel, !label.isHidden { result.append(label) }
            result += visibleLabels(in: sub)
        }
        return result
    }

    private func visibleLabelTags(in view: UIView) -> [Int] {
        var result: [Int] = []
        for sub in view.subviews {
            if let label = sub as? UILabel, !label.isHidden { result.append(label.tag) }
            result += visibleLabelTags(in: sub)
        }
        return result
    }
}
