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

    /// The rows the panel paints for the current selection, and where the selection
    /// sits within them. macOS opts into ADR-0013 paging at `pageSize` 10: the page is
    /// derived arithmetically from the selection (`selectedIndex / pageSize`), matching
    /// the session's own page math, so digit keys line up with the visible rows.
    ///
    /// Bounding the row count is also what keeps the panel positionable — an unbounded
    /// list grows taller than the space below the caret, and the screen clamp then
    /// overrides the caret anchor entirely.
    static func pageSlice<T>(
        candidates: [T],
        selectedIndex: Int,
        pageSize: Int
    ) -> (rows: [T], selectedRow: Int) {
        let size = max(1, pageSize)
        let index = max(0, min(selectedIndex, max(0, candidates.count - 1)))
        let start = (index / size) * size
        let end = min(start + size, candidates.count)
        guard start < end else { return ([], 0) }
        return (Array(candidates[start..<end]), index - start)
    }

    /// Panel width for the current page: fit the widest painted row, clamped to
    /// [minWidth, maxWidth]. Below the floor a one-char candidate would give too small a
    /// box; above the ceiling a long phrase would grow the panel past the caret anchor
    /// (it truncates instead). `widestRow` is the measured pixel width of the widest row's
    /// content including its own insets.
    static func contentWidth(widestRow: CGFloat, minWidth: CGFloat, maxWidth: CGFloat) -> CGFloat {
        max(minWidth, min(maxWidth, widestRow))
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
