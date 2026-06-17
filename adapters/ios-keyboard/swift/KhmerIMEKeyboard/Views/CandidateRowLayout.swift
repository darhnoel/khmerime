import CoreGraphics

// CandidateRowLayout
// ==================
// Pure horizontal-inset math for the candidate row. Centers the chips while they
// all fit the visible width (so a sparse set looks balanced), and falls back to
// the normal edge inset once they overflow — leaving the row left-aligned and
// scrollable.

enum CandidateRowLayout {
    static func centeringInset(contentWidth: CGFloat, availableWidth: CGFloat, edgeInset: CGFloat) -> CGFloat {
        let slack = availableWidth - contentWidth
        guard slack > 0 else { return edgeInset }
        return max(edgeInset, slack / 2)
    }
}
