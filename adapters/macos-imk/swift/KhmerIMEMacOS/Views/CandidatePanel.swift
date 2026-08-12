import AppKit
import os

// CandidatePanel
// ==============
// Non-activating floating NSPanel shown while composition is active.
// DISPLAY-ONLY: it renders MacosRenderState and accepts no mouse input —
// all interaction is keyboard-driven and interpreted by the Rust session
// (←/→ segment focus, ↑/↓ candidate highlight, 1–9 pick, Tab edit, Enter
// commit), exactly like the IBus lookup table on Linux.
//
// (Khmer spelling reminder: the -om sign is 'ុំ', never Roman 'uំ'.)
//
// Look: frosted glass (NSVisualEffectView) with rounded corners, like a native
// macOS popover/menu. Candidates are stacked VERTICALLY with 1-based index
// prefixes, mirroring the IBus vertical lookup table:
//
//   ┌──────────────────────────────┐
//   │  ណុំ   ទៅ   សាលារៀន          │  ← segment chips (focused = accent)
//   ├──────────────────────────────┤
//   │  1  ខ្ញុំ  nhom, nh, khnom     │  ← candidate rows (selected = accent fill)
//   │  2  ញ៉ុម  nhom, nhhom          │
//   │  3  ញ៉ុមៗ nhom, nhhom          │
//   └──────────────────────────────┘
//
// Positioning is delegated to CandidatePanelLayout (pure, unit-tested): the
// panel hangs below the caret, flips above when there is no room, and clamps to
// the screen — anchored with firstRectForCharacterRange: from the IMK client.
// Uses NSWindowStyleMask.nonActivatingPanel so it never steals keyboard focus;
// ignoresMouseEvents makes the display-only contract explicit.

final class CandidatePanel: NSPanel {

    // Dynamic width (fits the widest row, clamped to [minPanelWidth, maxPanelWidth]) and
    // dynamic height (grows with the candidate list).
    private let minPanelWidth: CGFloat = 180
    private let maxPanelWidth: CGFloat = 360
    private var panelWidth: CGFloat = 360
    private let rowHeight: CGFloat = 30
    private let candidateSpacing: CGFloat = 2
    private let topInset: CGFloat = 8
    private let bottomInset: CGFloat = 8

    private var rowContentWidth: CGFloat { panelWidth - 16 }

    /// The separator's width tracks the dynamic panel width; kept so `update` can retune it
    /// (the constant is baked at setup, unlike the rows which are rebuilt every update).
    private var separatorWidthConstraint: NSLayoutConstraint?

    // Frosted-glass backdrop
    private let effectView = NSVisualEffectView()

    // Vertical root: segment strip, separator, candidate list
    private let rootStack      = NSStackView()
    private let segmentStack   = NSStackView()
    private let separator      = NSBox()
    private let candidateStack = NSStackView()

    // MARK: - Init

    init() {
        let frame = NSRect(x: 0, y: 0, width: 360, height: 120)
        super.init(
            contentRect: frame,
            styleMask: [.nonactivatingPanel, .borderless],
            backing: .buffered,
            defer: false
        )
        isFloatingPanel    = true
        level              = .floating
        isOpaque           = false
        backgroundColor    = .clear
        hasShadow          = true
        ignoresMouseEvents = true
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        setupUI()
    }

    // MARK: - UI setup

    private func setupUI() {
        // ── Frosted glass content view ───────────────────────────────────────
        effectView.material        = .menu          // native frosted-menu look
        effectView.blendingMode    = .behindWindow  // blurs the desktop behind
        effectView.state           = .active
        effectView.wantsLayer      = true
        effectView.layer?.cornerRadius  = 10
        effectView.layer?.masksToBounds = true
        effectView.layer?.borderWidth   = 0.5
        effectView.layer?.borderColor   = NSColor.separatorColor.cgColor
        contentView = effectView

        // ── Segment chips (horizontal phrase preview) ────────────────────────
        segmentStack.orientation  = .horizontal
        segmentStack.spacing      = 6
        segmentStack.alignment    = .centerY
        segmentStack.edgeInsets   = NSEdgeInsets(top: 0, left: 8, bottom: 0, right: 8)
        segmentStack.translatesAutoresizingMaskIntoConstraints = false

        separator.boxType = .separator
        separator.translatesAutoresizingMaskIntoConstraints = false

        // ── Candidate list (vertical, IBus-style) ────────────────────────────
        candidateStack.orientation = .vertical
        candidateStack.spacing     = candidateSpacing
        candidateStack.alignment   = .leading
        candidateStack.edgeInsets  = NSEdgeInsets(top: 0, left: 4, bottom: 0, right: 4)
        candidateStack.translatesAutoresizingMaskIntoConstraints = false

        // ── Root ──────────────────────────────────────────────────────────────
        rootStack.orientation = .vertical
        rootStack.spacing     = 6
        rootStack.alignment   = .leading
        rootStack.translatesAutoresizingMaskIntoConstraints = false
        rootStack.addArrangedSubview(segmentStack)
        rootStack.addArrangedSubview(separator)
        rootStack.addArrangedSubview(candidateStack)

        effectView.addSubview(rootStack)
        let sepWidth = separator.widthAnchor.constraint(equalToConstant: rowContentWidth)
        separatorWidthConstraint = sepWidth
        NSLayoutConstraint.activate([
            rootStack.topAnchor.constraint(equalTo: effectView.topAnchor, constant: topInset),
            rootStack.leadingAnchor.constraint(equalTo: effectView.leadingAnchor, constant: 4),
            rootStack.trailingAnchor.constraint(equalTo: effectView.trailingAnchor, constant: -4),
            rootStack.bottomAnchor.constraint(equalTo: effectView.bottomAnchor, constant: -bottomInset),
            sepWidth,
        ])
    }

    // MARK: - Public API

    func update(_ state: MacosRenderState) {
        rebuildSegments(state.segments, mode: state.surfaceMode)
        let selectedIdx = state.selectedIndex.map { Int($0) } ?? 0

        // Paint one page, not the whole list (ADR-0013 — macOS opts in at page size 10).
        // The page is derived from the selection, so it flips as Space cycles past a
        // boundary and digit keys 1–9/0 line up with the visible rows. Bounding the row
        // count is also what keeps the panel small enough to sit below the caret.
        let page = CandidatePanelLayout.pageSlice(
            candidates: state.candidates,
            selectedIndex: selectedIdx,
            pageSize: Self.pageSize
        )
        let pageMetadata = CandidatePanelLayout.pageSlice(
            candidates: state.candidateDisplay,
            selectedIndex: selectedIdx,
            pageSize: Self.pageSize
        ).rows
        let entries = CandidateDisplayFormatter.displayEntries(
            candidates: page.rows,
            metadata: pageMetadata
        )
        // Size the panel to its widest painted content before laying rows out, so the row
        // width constraints (rebuilt below) pick up the clamped width.
        panelWidth = measuredPanelWidth(entries: entries, segments: state.segments, mode: state.surfaceMode)
        separatorWidthConstraint?.constant = rowContentWidth

        rebuildCandidates(entries, selectedIndex: page.selectedRow, mode: state.surfaceMode)

        let hasSegments = !state.segments.isEmpty
        segmentStack.isHidden = !hasSegments
        separator.isHidden    = !hasSegments
        resizeToFit(hasSegments: hasSegments, candidateCount: entries.count)
    }

    /// Rows painted per page. Must equal the session's `page_size` (ADR-0013
    /// constraint 4) or page-relative digit selection breaks.
    static let pageSize = 10

    func show(below caretRect: NSRect) {
        let screen = NSScreen.screens.first(where: { $0.frame.contains(caretRect.origin) })
            ?? NSScreen.main
        let visible = screen?.visibleFrame ?? caretRect
        let origin = CandidatePanelLayout.origin(
            caret: caretRect, panelSize: frame.size, screen: visible
        )
        // TEMP geometry diagnostics (no user text) — .notice so log show can
        // retrieve it; remove before commit.
        Logger(subsystem: "com.khmerime.inputmethod.KhmerIMEMacOS", category: "panel")
            .notice("[DEBUG-macos-imk-runtime] caret=\(NSStringFromRect(caretRect), privacy: .public) screen=\(NSStringFromRect(visible), privacy: .public) panel=\(NSStringFromSize(self.frame.size), privacy: .public) origin=\(NSStringFromPoint(origin), privacy: .public)")
        setFrameOrigin(origin)
        // orderFrontRegardless, not orderFront: the IME runs as a background
        // accessory (LSUIElement) and is never the active app, so orderFront(_:)
        // would be ignored and the panel would never appear.
        if !isVisible { orderFrontRegardless() }
    }

    func hide() {
        orderOut(nil)
    }

    // MARK: - Sizing

    /// Extra height for the roman sub-row under each Khmer header chip (ADR-0004).
    private let romanRowHeight: CGFloat = 15

    /// Panel width for the current content: the widest painted row (candidate rows and the
    /// two-row segment header), clamped to [minPanelWidth, maxPanelWidth]. Below the floor a
    /// short candidate would give too small a box; at the ceiling a long phrase truncates.
    private func measuredPanelWidth(entries: [MacosCandidateDisplayEntry],
                                    segments: [MacosSegmentEntry],
                                    mode: MacosSurfaceMode) -> CGFloat {
        let rowFont = NSFont.systemFont(ofSize: 15, weight: .regular)
        // Candidate row inner width: index column (16) + stack spacing (8) + text, inside the
        // row's own 8+8 insets. panelWidth = rowContentWidth + 16, so fold that back in below.
        let rowExtras: CGFloat = 16 + 8 + 8 + 8
        var widest: CGFloat = 0
        for entry in entries {
            let text = CandidateDisplayFormatter.displayText(for: entry, mode: mode)
            widest = max(widest, textWidth(text, font: rowFont) + rowExtras)
        }
        // Header: Khmer chips (15pt) laid out horizontally with 6pt gaps + 8+8 edge insets.
        // The roman sub-row is smaller, so the Khmer row bounds the header width.
        if !segments.isEmpty {
            let gaps = CGFloat(max(0, segments.count - 1)) * 6
            let chips = segments.reduce(CGFloat(0)) { $0 + textWidth($1.output, font: rowFont) }
            widest = max(widest, chips + gaps + 8 + 8 + 8)
        }
        // widest is content-width (rowContentWidth-scale); the panel adds 16 of chrome.
        return CandidatePanelLayout.contentWidth(
            widestRow: widest + 16, minWidth: minPanelWidth, maxWidth: maxPanelWidth
        )
    }

    private func textWidth(_ text: String, font: NSFont) -> CGFloat {
        (text as NSString).size(withAttributes: [.font: font]).width.rounded(.up)
    }

    private func resizeToFit(hasSegments: Bool, candidateCount: Int) {
        let segH: CGFloat = hasSegments ? rowHeight + romanRowHeight : 0
        let sepH: CGFloat = hasSegments ? (rootStack.spacing * 2 + 1) : 0
        let rows = max(candidateCount, 1)
        let listH = CGFloat(rows) * rowHeight + CGFloat(rows - 1) * candidateSpacing
        let total = topInset + bottomInset + segH + sepH + listH
        setContentSize(NSSize(width: panelWidth, height: total))
    }

    // MARK: - Segment chips

    private func rebuildSegments(_ segments: [MacosSegmentEntry], mode: MacosSurfaceMode) {
        segmentStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        // In Phrase mode the rows below ARE the whole-phrase candidates; the segmentation is only a
        // context header, so dim the chips (ADR-0004) — they must not read as a competing selection.
        // In Segment mode the chips show which word is being edited, so keep them prominent.
        let asContext = (mode == .phrase)
        for seg in segments {
            segmentStack.addArrangedSubview(makeChip(seg, asContext: asContext))
        }
    }

    private func makeChip(_ seg: MacosSegmentEntry, asContext: Bool) -> NSView {
        let label = makeLabel(seg.output)
        label.font = .systemFont(ofSize: 15, weight: seg.focused ? .semibold : .regular)
        // As a context header (Phrase mode), chips are secondary (dimmer than the selectable phrase
        // rows below, but still readable) and never accent-highlighted. Otherwise the focused segment
        // reads as accent.
        label.textColor = asContext ? .secondaryLabelColor : (seg.focused ? .controlAccentColor : .labelColor)
        label.drawsBackground = true
        label.backgroundColor = (!asContext && seg.focused)
            ? NSColor.controlAccentColor.withAlphaComponent(0.15)
            : .clear
        label.wantsLayer = true
        label.layer?.cornerRadius = 6
        label.lineBreakMode = .byTruncatingTail

        // Roman sub-row: the segment's roman (seg.input) sits directly under its Khmer, one column
        // per segment (ADR-0004). Dimmer than the Khmer above it. Empty roman → no sub-label so the
        // column just shows the Khmer.
        guard !seg.input.isEmpty else { return label }
        let roman = makeLabel(seg.input)
        roman.font = .systemFont(ofSize: 11, weight: .regular)
        roman.textColor = .tertiaryLabelColor
        roman.lineBreakMode = .byTruncatingTail

        let column = NSStackView(views: [label, roman])
        column.orientation = .vertical
        column.spacing = 1
        column.alignment = .leading
        return column
    }

    // MARK: - Candidate rows

    private func rebuildCandidates(_ candidates: [MacosCandidateDisplayEntry], selectedIndex: Int, mode: MacosSurfaceMode) {
        candidateStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        for (i, entry) in candidates.enumerated() {
            candidateStack.addArrangedSubview(
                makeCandidateRow(index: i + 1, entry: entry, selected: i == selectedIndex, mode: mode)
            )
        }
    }

    private func makeCandidateRow(index: Int, entry: MacosCandidateDisplayEntry, selected: Bool, mode: MacosSurfaceMode) -> NSView {
        let row = NSView()
        row.wantsLayer = true
        row.translatesAutoresizingMaskIntoConstraints = false
        row.layer?.cornerRadius = 5
        row.layer?.backgroundColor = selected
            ? NSColor.controlAccentColor.cgColor
            : NSColor.clear.cgColor

        // 1-based index prefix (IBus shows 1–9; blank past 9).
        let idxLabel = makeLabel(index <= 9 ? "\(index)" : "")
        idxLabel.font = .monospacedDigitSystemFont(ofSize: 12, weight: .regular)
        idxLabel.textColor = selected ? .white : .secondaryLabelColor

        let displayText = CandidateDisplayFormatter.displayText(for: entry, mode: mode)
        let textLabel = makeLabel(displayText)
        textLabel.font = .systemFont(ofSize: 15, weight: selected ? .semibold : .regular)
        // Base colour: white on the selected row, label colour otherwise. A red ✦ (unverified
        // model word, ADR-0016) must survive selection, so overlay it on top of the base colour.
        let baseColor: NSColor = selected ? .white : .labelColor
        if entry.fromModel {
            let attr = NSMutableAttributedString(
                string: displayText,
                attributes: [.foregroundColor: baseColor,
                             .font: NSFont.systemFont(ofSize: 15, weight: selected ? .semibold : .regular)]
            )
            if !entry.lexiconVerified {
                attr.addAttribute(.foregroundColor, value: NSColor.systemRed,
                                  range: NSRange(location: 0, length: 1)) // the leading ✦
            }
            textLabel.attributedStringValue = attr
        } else {
            textLabel.textColor = baseColor
        }
        textLabel.lineBreakMode = .byTruncatingTail
        textLabel.cell?.usesSingleLineMode = true

        let h = NSStackView(views: [idxLabel, textLabel])
        h.orientation = .horizontal
        h.spacing = 8
        h.alignment = .firstBaseline
        h.translatesAutoresizingMaskIntoConstraints = false
        row.addSubview(h)

        NSLayoutConstraint.activate([
            row.widthAnchor.constraint(equalToConstant: rowContentWidth),
            row.heightAnchor.constraint(equalToConstant: rowHeight),
            idxLabel.widthAnchor.constraint(equalToConstant: 16),
            h.leadingAnchor.constraint(equalTo: row.leadingAnchor, constant: 8),
            h.trailingAnchor.constraint(lessThanOrEqualTo: row.trailingAnchor, constant: -8),
            h.centerYAnchor.constraint(equalTo: row.centerYAnchor),
        ])
        return row
    }

    // MARK: - Label factory

    private func makeLabel(_ text: String) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.wantsLayer = true
        label.translatesAutoresizingMaskIntoConstraints = false
        return label
    }
}
