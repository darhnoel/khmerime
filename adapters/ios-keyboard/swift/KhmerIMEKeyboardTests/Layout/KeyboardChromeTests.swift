import XCTest
@testable import KhmerIMEKeyboard

// KeyboardChromeTests
// ===================
// The chrome (strip + candidate row) is only shown while there's something to
// display. isComposing decides expand vs collapse from the rendered content,
// not the keyboard state — so focusIn (empty render) collapses, and CharPick
// with candidates but no roman hint still expands.

final class KeyboardChromeTests: XCTestCase {

    func test_isComposing_emptyHintAndNoCandidates_isFalse() {
        let state = makeState(candidates: [])
        XCTAssertFalse(KeyboardChrome.isComposing(romanHint: "", state: state),
            "no roman hint and no candidates means nothing to show — chrome must collapse")
    }

    func test_isComposing_withRomanHint_isTrue() {
        let state = makeState(candidates: [])
        XCTAssertTrue(KeyboardChrome.isComposing(romanHint: "khn", state: state),
            "a non-empty roman hint fills the strip — chrome must expand")
    }

    func test_isComposing_emptyHintButCandidates_isTrue() {
        // CharPick: a letter is tapped, candidates populate the candidate row, but
        // the strip (roman hint) stays empty because CharPick doesn't touch romanBuffer.
        let state = makeState(candidates: ["ក", "ខ"])
        XCTAssertTrue(KeyboardChrome.isComposing(romanHint: "", state: state),
            "candidates fill the candidate row even with an empty strip — chrome must expand")
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
            segmentEditIndex: nil
        )
    }
}
