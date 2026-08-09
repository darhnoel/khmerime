import XCTest
@testable import KhmerIMEKeyboard

// KeyboardChromeTests
// ===================
// The chrome rows are shown only when a mode has content for them.

final class KeyboardChromeTests: XCTestCase {

    func test_quickAccessSpec_ownsExactDigitsAndAppleShapedMarks() {
        XCTAssertEqual(QuickAccessSpec.digits.map(\.commitText).joined(), "១២៣៤៥៦៧៨៩០")
        XCTAssertEqual(
            QuickAccessSpec.marks.map(\.commitText),
            ["។", "៕", "៖", "ៈ", "ៗ", "៘", "៙", "៚", "៛", "៊", "័", "៌", "៍", "៏", "៎", "៑"]
        )
        XCTAssertEqual(
            QuickAccessSpec.marks.map(\.displayText),
            ["។", "៕", "៖", "ៈ", "ៗ", "៘", "៙", "៚", "៛", "៊", "័", "៌", "៍", "៏", "៎", "៑"],
            "Apple's Khmer shaper supplies one placeholder circle for every isolated nonspacing mark"
        )
    }

    func test_presentation_followsMobileTwoOneZeroRowContract() {
        let empty = makeState(candidates: [])
        let charPickResults = makeState(candidates: ["ក", "ខ"])

        let presentations = [
            KeyboardChrome.presentation(isEnglish: false, keyboardState: .qwerty, romanHint: "", state: empty),
            KeyboardChrome.presentation(isEnglish: false, keyboardState: .numeric, romanHint: "", state: empty),
            KeyboardChrome.presentation(isEnglish: false, keyboardState: .symbols, romanHint: "", state: empty),
            KeyboardChrome.presentation(isEnglish: false, keyboardState: .qwerty, romanHint: "nhom", state: empty),
            KeyboardChrome.presentation(isEnglish: false, keyboardState: .charPick, romanHint: "", state: empty),
            KeyboardChrome.presentation(isEnglish: false, keyboardState: .charPick, romanHint: "", state: charPickResults),
            KeyboardChrome.presentation(isEnglish: true, keyboardState: .qwerty, romanHint: "", state: empty),
        ]

        XCTAssertEqual(
            presentations,
            [.quickAccess, .quickAccess, .quickAccess, .composition,
             .charPickQuickAccess, .charPickCandidates, .hidden]
        )
        XCTAssertEqual(presentations.map(\.rowCount), [2, 2, 2, 2, 1, 1, 0])
        XCTAssertEqual(
            presentations.map(\.rows),
            [.stripAndCandidate, .stripAndCandidate, .stripAndCandidate, .stripAndCandidate,
             .candidateOnly, .candidateOnly, .none]
        )
        XCTAssertEqual(
            KeyboardChrome.presentation(
                isEnglish: false, keyboardState: .numeric, romanHint: "nhom", state: empty
            ),
            .composition,
            "switching to 123 while composing must preserve composition chrome"
        )
    }

    // MARK: - helpers

    private func makeState(
        candidates: [String],
        phraseCandidates: [String] = [],
        selectedPhraseIndex: UInt64 = 0,
        segmentEditActive: Bool = false
    ) -> IosRenderState {
        IosRenderState(
            candidates: candidates,
            selectedIndex: nil,
            preedit: "",
            segments: [],
            focusedSegmentIndex: nil,
            commitText: nil,
            segmentEditActive: segmentEditActive,
            segmentEditIndex: nil,
            phraseCandidates: phraseCandidates.map {
                IosPhraseCandidate(text: $0, segments: [], fromModel: false, lexiconVerified: true)
            },
            selectedPhraseIndex: selectedPhraseIndex
        )
    }
}
