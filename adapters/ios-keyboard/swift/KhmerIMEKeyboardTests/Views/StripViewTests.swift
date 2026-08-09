import XCTest
import UIKit
@testable import KhmerIMEKeyboard

final class StripViewTests: XCTestCase {

    func test_showQuickAccess_displaysAllKhmerDigitsWithUniformRegularTypography() {
        let strip = StripView()

        strip.showQuickAccess(QuickAccessSpec.digits) { _ in }

        let digitLabels = visibleLabels(in: strip).filter { QuickAccessSpec.digits.map(\.displayText).contains($0.text ?? "") }
        let regularFontName = UIFont.systemFont(ofSize: 20, weight: .regular).fontName
        XCTAssertEqual(digitLabels.map(\.text), QuickAccessSpec.digits.map(\.displayText))
        XCTAssertEqual(digitLabels.map { $0.font.fontName }, Array(repeating: regularFontName, count: 10))
        XCTAssertEqual(digitLabels.map { $0.font.pointSize }, Array(repeating: 20, count: 10))
    }

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

    func test_selectedUnverifiedModelPhrase_coloursOnlyTheStripStarRed() {
        let strip = StripView()
        let state = IosRenderState(
            candidates: [], selectedIndex: nil, preedit: "gahebbadei",
            segments: [IosSegmentEntry(output: "គហិបតី", input: "gahebbadei", focused: false)],
            focusedSegmentIndex: nil, commitText: nil, segmentEditActive: false, segmentEditIndex: nil,
            phraseCandidates: [
                IosPhraseCandidate(text: "គហិបតី", segments: [], fromModel: true, lexiconVerified: false),
                IosPhraseCandidate(text: "គហបតី", segments: [], fromModel: true, lexiconVerified: true),
            ],
            selectedPhraseIndex: 0
        )

        strip.render(state, romanBuffer: "gahebbadei")

        let label = visibleLabels(in: strip).first { $0.attributedText?.string == "✦ គហិបតី" }
        XCTAssertEqual(label?.attributedText?.attribute(.foregroundColor, at: 0, effectiveRange: nil) as? UIColor,
            .systemRed, "the selected unverified Phrase Candidate needs the same red marker in the Strip")
        XCTAssertEqual(label?.attributedText?.attribute(.foregroundColor, at: 2, effectiveRange: nil) as? UIColor,
            .secondaryLabel, "the Khmer text itself must retain the Strip's normal colour")
    }

    private func visibleLabels(in view: UIView) -> [UILabel] {
        view.subviews.flatMap { subview -> [UILabel] in
            let current = (subview as? UILabel).map { [$0] } ?? []
            return current + visibleLabels(in: subview)
        }
    }
}
