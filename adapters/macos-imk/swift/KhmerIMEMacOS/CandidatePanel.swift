import AppKit

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
// Layout:
//   ┌───────────────────────────────────────────┐
//   │  [ណុំ]   ទៅ   សាលារៀន               44pt │  ← chips row (focused = accent)
//   ├───────────────────────────────────────────┤
//   │  ខ្ញុំ   ញុំ   ណុំ   ណ៉ំ  …            44pt │  ← candidates row (selected = accent)
//   └───────────────────────────────────────────┘
//
// Positioning: anchored just below the cursor using firstRectForCharacterRange:
// from the IMKTextInput client. Uses NSWindowStyleMask.nonActivatingPanel so
// the panel never steals keyboard focus from the host application;
// ignoresMouseEvents makes the display-only contract explicit.

final class CandidatePanel: NSPanel {

    // Chip row
    private let chipScrollView = NSScrollView()
    private let chipStack      = NSStackView()

    // Separator
    private let separator = NSBox()

    // Candidate row
    private let candidateScrollView = NSScrollView()
    private let candidateStack      = NSStackView()

    // MARK: - Init

    init() {
        let frame = NSRect(x: 0, y: 0, width: 480, height: 92)
        super.init(
            contentRect: frame,
            styleMask: [.nonactivatingPanel, .borderless],
            backing: .buffered,
            defer: false
        )
        isFloatingPanel  = true
        level            = .floating
        isOpaque         = false
        backgroundColor  = NSColor.windowBackgroundColor
        hasShadow        = true
        ignoresMouseEvents = true
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        setupUI()
    }

    // MARK: - UI setup

    private func setupUI() {
        guard let cv = contentView else { return }
        cv.wantsLayer = true
        cv.layer?.cornerRadius = 8
        cv.layer?.masksToBounds = true

        // ── Chip scroll ──────────────────────────────────────────────────────
        chipScrollView.hasHorizontalScroller       = false
        chipScrollView.hasVerticalScroller         = false
        chipScrollView.horizontalScrollElasticity  = .allowed
        chipScrollView.drawsBackground             = false
        chipScrollView.translatesAutoresizingMaskIntoConstraints = false

        chipStack.orientation   = .horizontal
        chipStack.spacing       = 8
        chipStack.alignment     = .centerY
        chipStack.distribution  = .gravityAreas
        chipStack.translatesAutoresizingMaskIntoConstraints = false
        chipScrollView.documentView = chipStack

        // ── Separator ────────────────────────────────────────────────────────
        separator.boxType = .separator
        separator.translatesAutoresizingMaskIntoConstraints = false

        // ── Candidate scroll ─────────────────────────────────────────────────
        candidateScrollView.hasHorizontalScroller      = false
        candidateScrollView.hasVerticalScroller        = false
        candidateScrollView.horizontalScrollElasticity = .allowed
        candidateScrollView.drawsBackground            = false
        candidateScrollView.translatesAutoresizingMaskIntoConstraints = false

        candidateStack.orientation  = .horizontal
        candidateStack.spacing      = 6
        candidateStack.alignment    = .centerY
        candidateStack.distribution = .gravityAreas
        candidateStack.translatesAutoresizingMaskIntoConstraints = false
        candidateScrollView.documentView = candidateStack

        for v in [chipScrollView, separator, candidateScrollView] as [NSView] {
            cv.addSubview(v)
        }

        NSLayoutConstraint.activate([
            chipScrollView.topAnchor.constraint(equalTo: cv.topAnchor),
            chipScrollView.leadingAnchor.constraint(equalTo: cv.leadingAnchor),
            chipScrollView.trailingAnchor.constraint(equalTo: cv.trailingAnchor),
            chipScrollView.heightAnchor.constraint(equalToConstant: 44),

            separator.topAnchor.constraint(equalTo: chipScrollView.bottomAnchor),
            separator.leadingAnchor.constraint(equalTo: cv.leadingAnchor),
            separator.trailingAnchor.constraint(equalTo: cv.trailingAnchor),

            candidateScrollView.topAnchor.constraint(equalTo: separator.bottomAnchor),
            candidateScrollView.leadingAnchor.constraint(equalTo: cv.leadingAnchor),
            candidateScrollView.trailingAnchor.constraint(equalTo: cv.trailingAnchor),
            candidateScrollView.bottomAnchor.constraint(equalTo: cv.bottomAnchor),
            candidateScrollView.heightAnchor.constraint(equalToConstant: 44),
        ])
    }

    // MARK: - Public API

    func update(_ state: MacosRenderState) {
        rebuildChips(state.segments)
        let selectedIdx = state.selectedIndex.map { Int($0) } ?? 0
        rebuildCandidates(state.candidates, selectedIndex: selectedIdx)
    }

    func show(below cursorRect: NSRect) {
        let origin = NSPoint(
            x: cursorRect.origin.x,
            y: cursorRect.origin.y - frame.height - 4
        )
        setFrameOrigin(origin)
        if !isVisible { orderFront(nil) }
    }

    func hide() {
        orderOut(nil)
    }

    // MARK: - Chip row

    private func rebuildChips(_ segments: [MacosSegmentEntry]) {
        chipStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        for seg in segments {
            chipStack.addArrangedSubview(makeChip(seg))
        }
        chipScrollView.documentView?.frame.size.width =
            max(chipScrollView.frame.width, chipStack.fittingSize.width + 16)
    }

    private func makeChip(_ seg: MacosSegmentEntry) -> NSView {
        let label = makeLabel(seg.output)
        label.font = .systemFont(ofSize: 15, weight: seg.focused ? .semibold : .regular)
        label.textColor = seg.focused ? .controlAccentColor : .labelColor
        label.layer?.backgroundColor = seg.focused
            ? NSColor.controlAccentColor.withAlphaComponent(0.15).cgColor
            : NSColor.controlBackgroundColor.cgColor
        label.layer?.cornerRadius = 10
        return label
    }

    // MARK: - Candidate row

    private func rebuildCandidates(_ candidates: [String], selectedIndex: Int) {
        candidateStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        for (i, text) in candidates.enumerated() {
            candidateStack.addArrangedSubview(makeCandidate(text, selected: i == selectedIndex))
        }
        candidateScrollView.documentView?.frame.size.width =
            max(candidateScrollView.frame.width, candidateStack.fittingSize.width + 16)
    }

    private func makeCandidate(_ text: String, selected: Bool) -> NSView {
        let label = makeLabel(text)
        label.font = .systemFont(ofSize: 18, weight: selected ? .semibold : .medium)
        label.textColor = selected ? .controlAccentColor : .labelColor
        label.layer?.backgroundColor = selected
            ? NSColor.controlAccentColor.withAlphaComponent(0.1).cgColor
            : NSColor.clear.cgColor
        label.layer?.cornerRadius = 6
        return label
    }

    // MARK: - Label factory

    private func makeLabel(_ text: String) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.wantsLayer = true
        label.alignment = .center
        label.translatesAutoresizingMaskIntoConstraints = false
        return label
    }
}
