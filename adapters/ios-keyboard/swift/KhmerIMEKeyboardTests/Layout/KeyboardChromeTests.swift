import XCTest
@testable import KhmerIMEKeyboard

// KeyboardChromeTests
// ===================
// The chrome rows are shown only when a mode has content for them. Roman
// composition owns the strip + candidate row; CharPick owns only the candidate
// row.

final class KeyboardChromeTests: XCTestCase {

    func test_rows_qwertyEmptyHintAndNoCandidates_isNone() {
        let state = makeState(candidates: [])

        XCTAssertEqual(KeyboardChrome.rows(for: .qwerty, romanHint: "", state: state), .none,
            "no roman hint and no candidates means nothing to show — chrome must collapse")
    }

    func test_rows_qwertyWithRomanHint_showsStripAndCandidateRows() {
        let state = makeState(candidates: [])

        XCTAssertEqual(KeyboardChrome.rows(for: .qwerty, romanHint: "khn", state: state), .stripAndCandidate,
            "a non-empty roman hint fills the strip, so roman composition keeps the full chrome")
    }

    func test_rows_qwertyEmptyHintButCandidates_showsStripAndCandidateRows() {
        let state = makeState(candidates: ["ក", "ខ"])

        XCTAssertEqual(KeyboardChrome.rows(for: .qwerty, romanHint: "", state: state), .stripAndCandidate,
            "non-CharPick candidate browsing still reserves the full composition chrome")
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

    private func makeState(candidates: [String]) -> IosRenderState {
        IosRenderState(
            candidates: candidates,
            selectedIndex: nil,
            preedit: "",
            segments: [],
            focusedSegmentIndex: nil,
            commitText: nil,
            segmentEditActive: false,
            segmentEditIndex: nil,
            phraseCandidates: []
        )
    }
}
