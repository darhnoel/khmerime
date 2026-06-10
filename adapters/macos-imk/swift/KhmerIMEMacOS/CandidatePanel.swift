import AppKit

// CandidatePanel
// ==============
// Non-activating floating NSPanel shown while composition is active.
//
// Layout (mirrors CandidatePanelView on iOS — see ADR-0003):
//
//   ┌───────────────────────────────────────────┐
//   │  [ណuំ ✏]  [ទៅ ✏]  [សាលារៀន ✏]         44pt │  ← chips row (NSScrollView)
//   ├───────────────────────────────────────────┤
//   │  ខ្ញuំ   ញuំ   ណuំ   ណ៉ំ  …             44pt │  ← candidates row (NSScrollView)
//   └───────────────────────────────────────────┘
//
// Positioning: anchored just below the cursor using firstRectForCharacterRange:
// from the IMKTextInput client. Uses NSWindowStyleMask.nonActivatingPanel so
// the panel never steals keyboard focus from the host application.

final class CandidatePanel: NSPanel {

    private weak var controller: KhmerInputController?

    // Chip row
    private let chipScrollView = NSScrollView()
    private let chipStack      = NSStackView()

    // Separator
    private let separator = NSBox()

    // Candidate row
    private let candidateScrollView = NSScrollView()
    private let candidateStack      = NSStackView()

    // MARK: - Init

    init(controller: KhmerInputController) {
        self.controller = controller
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
        separator.boxType           = .separator
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

    // MARK: - Rebuild chip row

    private func rebuildChips(_ segments: [MacosSegmentEntry]) {
        chipStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        for (i, seg) in segments.enumerated() {
            chipStack.addArrangedSubview(makeChip(seg, index: i))
        }
        chipScrollView.documentView?.frame.size.width =
            max(chipScrollView.frame.width, chipStack.fittingSize.width + 16)
    }

    private func makeChip(_ seg: MacosSegmentEntry, index: Int) -> NSView {
        let container = NSView()
        container.translatesAutoresizingMaskIntoConstraints = false

        let chip = NSButton(title: seg.output, target: self, action: #selector(chipTapped(_:)))
        chip.tag = index
        chip.isBordered = false
        chip.wantsLayer = true
        chip.layer?.cornerRadius = 10
        chip.layer?.backgroundColor = seg.focused
            ? NSColor.controlAccentColor.withAlphaComponent(0.15).cgColor
            : NSColor.controlBackgroundColor.cgColor
        chip.font = .systemFont(ofSize: 15, weight: seg.focused ? .semibold : .regular)
        chip.contentTintColor = seg.focused ? .controlAccentColor : .labelColor
        chip.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(chip)

        let editBtn = NSButton(title: "✏", target: self, action: #selector(editTapped(_:)))
        editBtn.tag = index
        editBtn.isBordered = false
        editBtn.font = .systemFont(ofSize: 11)
        editBtn.contentTintColor = .controlAccentColor
        editBtn.isHidden = !seg.focused
        editBtn.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(editBtn)

        NSLayoutConstraint.activate([
            chip.topAnchor.constraint(equalTo: container.topAnchor, constant: 4),
            chip.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 4),
            chip.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -4),

            editBtn.leadingAnchor.constraint(equalTo: chip.trailingAnchor, constant: 2),
            editBtn.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -4),
            editBtn.centerYAnchor.constraint(equalTo: container.centerYAnchor),
            editBtn.widthAnchor.constraint(equalToConstant: 18),
        ])
        return container
    }

    // MARK: - Rebuild candidate row

    private func rebuildCandidates(_ candidates: [String], selectedIndex: Int) {
        candidateStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        for (i, text) in candidates.enumerated() {
            candidateStack.addArrangedSubview(makeCandidateButton(text: text, index: i, selected: i == selectedIndex))
        }
        candidateScrollView.documentView?.frame.size.width =
            max(candidateScrollView.frame.width, candidateStack.fittingSize.width + 16)
    }

    private func makeCandidateButton(text: String, index: Int, selected: Bool) -> NSButton {
        let btn = NSButton(title: text, target: self, action: #selector(candidateTapped(_:)))
        btn.tag = index
        btn.isBordered = false
        btn.wantsLayer = true
        btn.layer?.cornerRadius = 6
        btn.layer?.backgroundColor = selected
            ? NSColor.controlAccentColor.withAlphaComponent(0.1).cgColor
            : NSColor.clear.cgColor
        btn.font = .systemFont(ofSize: 18, weight: selected ? .semibold : .medium)
        btn.contentTintColor = selected ? .controlAccentColor : .labelColor
        return btn
    }

    // MARK: - Actions

    @objc private func chipTapped(_ sender: NSButton) {
        controller?.panelDidTapChip(at: sender.tag)
    }

    @objc private func editTapped(_ sender: NSButton) {
        controller?.panelDidRequestEdit(at: sender.tag)
    }

    @objc private func candidateTapped(_ sender: NSButton) {
        controller?.panelDidSelectCandidate(at: sender.tag)
    }
}
