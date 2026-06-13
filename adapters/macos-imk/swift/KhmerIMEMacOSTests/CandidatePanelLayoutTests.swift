import XCTest

// CandidatePanelLayoutTests
// =========================
// Pure geometry — the panel must sit clear of the caret line so it never hides
// what the user is typing. Cocoa screen coords: origin bottom-left, y up.

final class CandidatePanelLayoutTests: XCTestCase {

    // A roomy screen so default placement is never forced to flip/clamp.
    private let screen = CGRect(x: 0, y: 0, width: 1440, height: 900)
    private let panel = CGSize(width: 480, height: 120)

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
