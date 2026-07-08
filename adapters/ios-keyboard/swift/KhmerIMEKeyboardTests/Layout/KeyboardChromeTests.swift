import XCTest
@testable import KhmerIMEKeyboard

// KeyboardChromeTests
// ===================
// The chrome rows are shown only when a mode has content for them.

final class KeyboardChromeTests: XCTestCase {

    func test_rows_qwertyEmptyHintAndNoContent_isNone() {
        let state = makeState(candidates: [])

        XCTAssertEqual(KeyboardChrome.rows(for: .qwerty, romanHint: "", state: state), .none,
            "no roman hint and no render content means chrome must collapse")
    }

    func test_rows_qwertyWithRomanHintButNoPhraseAlternatives_showsStripOnly() {
        let state = makeState(candidates: [])

        XCTAssertEqual(KeyboardChrome.rows(for: .qwerty, romanHint: "khn", state: state), .stripOnly,
            "a non-empty roman hint fills the strip, but an empty Phrase Wheel must not reserve its row")
    }

    func test_rows_qwertyWithPhraseAlternatives_showsStripAndCandidateRows() {
        let state = makeState(candidates: [], phraseCandidates: ["ខ្ញុំ", "ញ៉ម"])

        XCTAssertEqual(KeyboardChrome.rows(for: .qwerty, romanHint: "khnhom", state: state), .stripAndCandidate,
            "normal composition reserves the candidate row only when the Phrase Wheel has an alternative")
    }

    func test_rows_qwertyExcludesSelectedPhraseWhenCheckingAlternatives() {
        let state = makeState(candidates: [], phraseCandidates: ["ខ្ញុំ", "ញ៉ម", "ញំ"], selectedPhraseIndex: 1)

        XCTAssertEqual(KeyboardChrome.rows(for: .qwerty, romanHint: "khnhom", state: state), .stripAndCandidate,
            "after selecting an alternative, the original best counts as a visible wheel alternative")
    }

    func test_rows_qwertyIgnoresWordCandidatesOutsideSegmentEdit() {
        let state = makeState(candidates: ["ក", "ខ"])

        XCTAssertEqual(KeyboardChrome.rows(for: .qwerty, romanHint: "k", state: state), .stripOnly,
            "default composition uses the Phrase Wheel, so word candidates alone should not reserve the row")
    }

    func test_rows_segmentEditWithCandidates_showsStripAndCandidateRows() {
        let state = makeState(candidates: ["ក", "ខ"], segmentEditActive: true)

        XCTAssertEqual(KeyboardChrome.rows(for: .qwerty, romanHint: "k", state: state), .stripAndCandidate,
            "Segment Edit uses word candidates in the candidate row")
    }

    func test_rows_segmentEditWithoutCandidates_showsStripOnly() {
        let state = makeState(candidates: [], segmentEditActive: true)

        XCTAssertEqual(KeyboardChrome.rows(for: .qwerty, romanHint: "k", state: state), .stripOnly,
            "Segment Edit should not reserve an empty word-candidate row")
    }

    func test_rows_charPickWithoutCandidates_isNone() {
        let state = makeState(candidates: [])

        XCTAssertEqual(KeyboardChrome.rows(for: .charPick, romanHint: "", state: state), .none,
            "entering CharPick alone should not reserve an empty row")
    }

    func test_rows_charPickWithCandidates_showsCandidateRowOnly() {
        let state = makeState(candidates: ["ក", "ខ"])

        XCTAssertEqual(KeyboardChrome.rows(for: .charPick, romanHint: "", state: state), .candidateOnly,
            "CharPick candidates need the candidate row, not the roman strip")
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
            phraseCandidates: phraseCandidates.map { IosPhraseCandidate(text: $0, segments: []) },
            selectedPhraseIndex: selectedPhraseIndex
        )
    }
}
