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

    func test_zeroHeightCaretStillClearsTheLine() {
        // Some hosts report a zero-height caret. The line-clearance drop must be
        // floored to a minimum line height (18) so the panel can't ride up onto
        // the typing line when the caret reports no height.
        let caret = CGRect(x: 200, y: 500, width: 2, height: 0)

        let origin = CandidatePanelLayout.origin(
            caret: caret, panelSize: panel, screen: screen
        )

        let panelTop = origin.y + panel.height
        XCTAssertGreaterThanOrEqual(caret.minY - panelTop, 18,
            "with a zero-height caret the panel must still drop a full floored line below the caret")
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

    func test_caretAnchorsAtEndOfPreedit_soPanelFollowsCursor() {
        // The caret sits at the END of the marked text, so the panel must query
        // the rect there — not at offset 0, which stays pinned to where the
        // composition began and makes the panel look stuck while typing.
        XCTAssertEqual(
            CandidatePanelLayout.caretQueryRange(preedit: "nhom"),
            NSRange(location: 4, length: 0)
        )
        // utf16 count, not Character count: a Khmer cluster spans several units.
        XCTAssertEqual(
            CandidatePanelLayout.caretQueryRange(preedit: "ខ្ញុំ"),
            NSRange(location: ("ខ្ញុំ" as NSString).length, length: 0)
        )
    }

    func test_caretAnchorRangeTargetsLastGlyphInsideTheMarkedText() {
        // Anchor at the LAST glyph (a range INSIDE the marked text), not one past
        // the end — querying {length, 0} is out of range and the host answers it
        // with a degenerate origin rect (the left-margin bug).
        XCTAssertEqual(
            CandidatePanelLayout.caretAnchorRange(preedit: "nhom"),
            NSRange(location: 3, length: 1)               // the 'm'
        )
        XCTAssertEqual(
            CandidatePanelLayout.caretAnchorRange(preedit: "f"),
            NSRange(location: 0, length: 1)               // single glyph
        )
        // Empty preedit → the {0,0} insertion point.
        XCTAssertEqual(
            CandidatePanelLayout.caretAnchorRange(preedit: ""),
            NSRange(location: 0, length: 0)
        )
        // utf16 units: the last code unit of a multi-scalar Khmer cluster.
        let kh = "ខ្ញុំ"
        XCTAssertEqual(
            CandidatePanelLayout.caretAnchorRange(preedit: kh),
            NSRange(location: (kh as NSString).length - 1, length: 1)
        )
    }

    func test_caretPointSitsAtTrailingEdgeOfLastGlyph() {
        // The caret is the trailing (right) edge of the last marked glyph, with
        // zero width, preserving the glyph's vertical extent — so the panel
        // anchors at the END of the composition where the cursor actually is.
        let glyph = CGRect(x: 100, y: 500, width: 12, height: 18)

        let caret = CandidatePanelLayout.caretPoint(fromGlyphRect: glyph)

        XCTAssertEqual(caret, CGRect(x: 112, y: 500, width: 0, height: 18))
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
