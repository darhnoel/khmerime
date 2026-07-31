import XCTest

// CandidatePanelLayoutTests
// =========================
// Pure geometry — the panel must sit clear of the caret line so it never hides
// what the user is typing. Cocoa screen coords: origin bottom-left, y up.

final class CandidatePanelLayoutTests: XCTestCase {

    // A roomy screen so default placement is never forced to flip/clamp.
    private let screen = CGRect(x: 0, y: 0, width: 1440, height: 900)
    private let panel = CGSize(width: 480, height: 120)

    // ADR-0013 paging, macOS opting in at page_size 10. Painting every candidate made
    // the panel tall enough that it could not fit below the caret, so the screen clamp
    // overrode the caret anchor and parked it mid-screen.
    func test_pageSlice_showsOnlyTheSelectedCandidatesPage() {
        let all = (0..<21).map { "c\($0)" }

        let firstPage = CandidatePanelLayout.pageSlice(candidates: all, selectedIndex: 0, pageSize: 10)

        XCTAssertEqual(firstPage.rows, Array(all[0..<10]),
            "a 21-candidate list must paint only the first 10 rows")
        XCTAssertEqual(firstPage.selectedRow, 0,
            "the selected candidate's row is page-relative")
    }

    // Space cycles the selection one at a time; when it crosses a page boundary the
    // painted page flips with it (ADR-0013 — pagination emerges from cursor movement,
    // there is no separate page key).
    func test_pageSlice_flipsToTheSecondPageWhenSelectionCrossesTheBoundary() {
        let all = (0..<21).map { "c\($0)" }

        let secondPage = CandidatePanelLayout.pageSlice(candidates: all, selectedIndex: 10, pageSize: 10)

        XCTAssertEqual(secondPage.rows, Array(all[10..<20]),
            "selecting index 10 must flip to the second page")
        XCTAssertEqual(secondPage.selectedRow, 0,
            "index 10 is the first row of page two")
    }

    // The raw roman fallback is the last candidate (ADR-0013). A final short page must
    // paint only what exists rather than padding or overrunning.
    func test_pageSlice_lastPageIsShortNotPadded() {
        let all = (0..<21).map { "c\($0)" }

        let lastPage = CandidatePanelLayout.pageSlice(candidates: all, selectedIndex: 20, pageSize: 10)

        XCTAssertEqual(lastPage.rows, ["c20"],
            "the 21st candidate sits alone on a short final page")
        XCTAssertEqual(lastPage.selectedRow, 0)
    }

    func test_panelSitsBelowCaretWithoutOverlappingIt() {
        // Caret mid-screen with a real line height.
        let caret = CGRect(x: 200, y: 500, width: 2, height: 18)

        let origin = CandidatePanelLayout.origin(
            caret: caret, panelSize: panel, screen: screen
        )

        let panelTop = origin.y + panel.height
        XCTAssertLessThanOrEqual(panelTop, caret.minY,
            "panel top must be at or below the caret bottom — it must not cover the typing line")
        XCTAssertEqual(origin.x, caret.minX,
            "panel left edge aligns with the caret by default")
    }

    func test_panelFlipsAboveCaretWhenNoRoomBelow() {
        // Caret near the bottom of the screen: hanging the panel below would push
        // it off the bottom edge, so it must flip to sit above the caret instead.
        let caret = CGRect(x: 200, y: 40, width: 2, height: 18)

        let origin = CandidatePanelLayout.origin(
            caret: caret, panelSize: panel, screen: screen
        )

        XCTAssertGreaterThanOrEqual(origin.y, caret.maxY,
            "panel must flip above the caret when there is no room below")
        XCTAssertLessThanOrEqual(origin.y + panel.height, screen.maxY,
            "flipped panel must still fit on screen")
    }

    func test_panelTallerThanFitKeepsTopRowsOnScreen() {
        // Dead zone: caret where the panel fits neither below nor (flipped) above.
        // The final clamp must keep the TOP of the panel (rows 1–9, the selected
        // candidate) on screen, sacrificing the bottom of a long list instead.
        let shortScreen = CGRect(x: 0, y: 0, width: 1440, height: 300)
        let tallPanel = CGSize(width: 480, height: 350)
        let caret = CGRect(x: 200, y: 120, width: 2, height: 18)

        let origin = CandidatePanelLayout.origin(
            caret: caret, panelSize: tallPanel, screen: shortScreen
        )

        XCTAssertLessThanOrEqual(origin.y + tallPanel.height, shortScreen.maxY,
            "the panel top (rows 1–9) must never be clipped off the top of the screen")
    }

    func test_caretAnchorIndexTargetsLastGlyphOfPreedit() {
        // Anchor on the LAST glyph so the line rect tracks the end of the
        // composition; an empty preedit uses the index-0 insertion point.
        XCTAssertEqual(CandidatePanelLayout.caretAnchorIndex(preedit: "nhom"), 3)
        XCTAssertEqual(CandidatePanelLayout.caretAnchorIndex(preedit: "f"), 0)
        XCTAssertEqual(CandidatePanelLayout.caretAnchorIndex(preedit: ""), 0)
        // utf16 units: the last code unit of a multi-scalar Khmer cluster.
        let kh = "ខ្ញុំ"
        XCTAssertEqual(
            CandidatePanelLayout.caretAnchorIndex(preedit: kh),
            (kh as NSString).length - 1
        )
    }

    func test_panelClampsHorizontallyToStayOnScreen() {
        // Caret near the right edge: aligning the panel's left with the caret
        // would overflow the right edge, so it must shift left to stay on screen.
        let caret = CGRect(x: 1400, y: 500, width: 2, height: 18)

        let origin = CandidatePanelLayout.origin(
            caret: caret, panelSize: panel, screen: screen
        )

        XCTAssertLessThanOrEqual(origin.x + panel.width, screen.maxX,
            "panel right edge must not overflow the screen")
        XCTAssertGreaterThanOrEqual(origin.x, screen.minX,
            "panel left edge must not fall off the screen")
    }
}
