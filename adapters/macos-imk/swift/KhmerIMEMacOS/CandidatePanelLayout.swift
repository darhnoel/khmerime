import CoreGraphics
import Foundation

// CandidatePanelLayout
// ====================
// Pure geometry for placing the candidate panel relative to the text caret.
// No AppKit: takes plain CGRect/CGSize so it is unit-testable without a screen.
//
// Cocoa screen coordinates: origin bottom-left, y increases upward. "Below the
// caret" therefore means a SMALLER y. The caret rect is the line-height
// rectangle reported by attributes(forCharacterIndex:lineHeightRectangle:) — a
// true line rect, so hanging the panel `gap` below its bottom already clears the
// line. If there is not enough room below, it flips above; the result is clamped
// to the visible screen so a degenerate rect cannot strand it off-screen.

enum CandidatePanelLayout {

    static func origin(
        caret: CGRect,
        panelSize: CGSize,
        screen: CGRect,
        gap: CGFloat = 6
    ) -> CGPoint {
        // Hang the panel just below the caret line. caret is a real line rect, so
        // its bottom (minY) is the line's bottom; `gap` below it clears the line.
        let below = caret.minY - gap - panelSize.height
        // If that pushes the panel off the bottom edge, flip it above the caret.
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

    /// The marked-text character index to ask the IMK client for via
    /// attributes(forCharacterIndex:lineHeightRectangle:). Targets the LAST glyph
    /// of the preedit so the line rect tracks the end of the composition (the
    /// panel follows the caret as the user types). An empty preedit uses index 0,
    /// the insertion point. Index is in UTF-16 units, the unit IMK uses.
    static func caretAnchorIndex(preedit: String) -> Int {
        max(0, (preedit as NSString).length - 1)
    }
}
