import CoreGraphics
import Foundation

// CandidatePanelLayout
// ====================
// Pure geometry for placing the candidate panel relative to the text caret.
// No AppKit: takes plain CGRect/CGSize so it is unit-testable without a screen.
//
// Cocoa screen coordinates: origin bottom-left, y increases upward. "Below the
// caret" therefore means a SMALLER y. The panel must never overlap the caret
// line (that hides what the user is typing), so by default it hangs just under
// the caret; if there is not enough room above the screen's bottom edge it
// flips to sit above the caret instead. The result is always clamped to the
// visible screen so a degenerate caret rect cannot strand it off-screen.

enum CandidatePanelLayout {

    /// Floor for the line-clearance drop: some hosts report a zero-height caret
    /// rect, and without a floor the panel would ride back up onto the line.
    static let minLineHeight: CGFloat = 18

    static func origin(
        caret: CGRect,
        panelSize: CGSize,
        screen: CGRect,
        gap: CGFloat = 6
    ) -> CGPoint {
        // Hang the panel a full line below the caret: drop by the line height as
        // well as `gap`, because firstRectForCharacterRange: for marked text
        // anchors the rect at the line's TOP, so subtracting only `gap` leaves
        // the panel sitting on the typing line. Subtracting the height clears the
        // whole line (and any Khmer subscripts) regardless of that anchoring.
        // The height is floored so a zero-height caret can't collapse the drop.
        let lineHeight = max(caret.height, minLineHeight)
        let below = caret.minY - lineHeight - gap - panelSize.height
        // If that pushes the panel off the bottom edge, flip it above the caret.
        // The top-anchoring quirk needs no compensation here — caret.maxY already
        // sits above the visual line, so `+ gap` clears it.
        var y = below < screen.minY ? caret.maxY + gap : below
        // Final vertical clamp for the dead zone where the panel fits neither
        // below nor above. Top-biased: the bottom clamp is applied first and the
        // top clamp last, so when the panel is taller than the available space we
        // keep its TOP (rows 1–9, the selected candidate) on screen and let the
        // bottom of a long list fall off, rather than clipping the rows in use.
        y = max(y, screen.minY)
        y = min(y, screen.maxY - panelSize.height)
        // Keep the panel within the screen's horizontal bounds.
        let x = min(max(caret.minX, screen.minX), screen.maxX - panelSize.width)
        return CGPoint(x: x, y: y)
    }

    /// The marked-text range to ask the IMK client for via
    /// firstRectForCharacterRange:. The caret sits at the END of the preedit, so
    /// anchoring there makes the panel follow the cursor while typing instead of
    /// staying pinned to where composition began. Length is in UTF-16 units (the
    /// unit IMK ranges use), which is why a multi-scalar Khmer cluster counts as
    /// more than one.
    static func caretQueryRange(preedit: String) -> NSRange {
        NSRange(location: (preedit as NSString).length, length: 0)
    }
}
