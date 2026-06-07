import UIKit

// MARK: - Delegate

// CandidatePanelDelegate
// ======================
// Notified by CandidatePanelView on every user interaction. The owner
// (KeyboardViewController) implements this to forward events to the session.
//
// Android equivalent
// ------------------
// Define a Kotlin interface (listener) with the same four callbacks and pass
// it to the CandidatePanelView during construction:
//
//   interface CandidatePanelListener {
//       fun onChipTapped(index: Int)
//       fun onEditChipTapped(index: Int)
//       fun onCandidateSelected(index: Int)
//       fun onDismiss()
//   }

protocol CandidatePanelDelegate: AnyObject {
    /// User tapped a segment chip to move focus to that segment.
    func candidatePanel(_ panel: CandidatePanelView, didTapChipAt index: Int)
    /// User tapped ✏ on a chip to enter Segment Edit Mode for that segment.
    func candidatePanel(_ panel: CandidatePanelView, didRequestEditAt index: Int)
    /// User tapped a candidate to select it for the focused segment.
    func candidatePanel(_ panel: CandidatePanelView, didSelectCandidateAt index: Int)
    /// User tapped ⊞ to dismiss the panel and return to the QWERTY view.
    func candidatePanelDidDismiss(_ panel: CandidatePanelView)
}

// MARK: - View

// CandidatePanelView
// ==================
// Full-replacement view for the key-rows area, shown when the user taps ⊞.
// The strip (StripView) remains above it at all times.
//
// Android equivalent
// ------------------
// A ConstraintLayout replacing the key rows:
//
//   <ConstraintLayout>
//       <HorizontalScrollView android:id="@+id/chipScroll" />  <!-- chips row -->
//       <View android:id="@+id/sep1" android:layout_height="0.5dp" />
//       <HorizontalScrollView android:id="@+id/candScroll" />  <!-- candidates -->
//       <View android:id="@+id/sep2" android:layout_height="0.5dp" />
//       <!-- bottom row comes from the shared layout -->
//   </ConstraintLayout>
//
//   fun render(state: IosRenderState) { rebuildChips(…); rebuildCandidates(…) }
//
// Layout (replaces key rows; strip stays above):
//   ┌───────────────────────────────────────────┐
//   │  [⊞]  [ណuំ ✏]  [ទៅ ✏]  [សាលារៀន ✏]  44pt │  ← chips (scrollable)
//   ├───────────────────────────────────────────┤
//   │  ខ្ញuំ   ញuំ   ណuំ   ណ៉ំ  …             52pt │  ← candidates (scrollable)
//   ├───────────────────────────────────────────┤
//   │  123  │      space      │  .  │    ⏎     │  ← bottom row (from VC)
//   └───────────────────────────────────────────┘

final class CandidatePanelView: UIView {

    weak var delegate: CandidatePanelDelegate?

    // Exposed so the ViewController can embed the shared bottom row.
    let bottomAnchorGuide = UILayoutGuide()

    private let chipScroll      = UIScrollView()
    private let chipStack       = UIStackView()
    private let candidateScroll = UIScrollView()
    private let candidateStack  = UIStackView()

    override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) { fatalError("use init(frame:)") }

    // MARK: - Setup

    private func setup() {
        chipScroll.showsHorizontalScrollIndicator = false
        chipScroll.backgroundColor = .systemBackground
        chipScroll.translatesAutoresizingMaskIntoConstraints = false
        chipScroll.delaysContentTouches = false

        chipStack.axis = .horizontal
        chipStack.spacing = 8
        chipStack.alignment = .center
        chipStack.translatesAutoresizingMaskIntoConstraints = false
        chipScroll.addSubview(chipStack)

        let chipSeparator = makeSeparator()

        candidateScroll.showsHorizontalScrollIndicator = false
        candidateScroll.backgroundColor = UIColor.systemGray6
        candidateScroll.translatesAutoresizingMaskIntoConstraints = false
        candidateScroll.delaysContentTouches = false

        candidateStack.axis = .horizontal
        candidateStack.spacing = 6
        candidateStack.alignment = .center
        candidateStack.translatesAutoresizingMaskIntoConstraints = false
        candidateScroll.addSubview(candidateStack)

        let candidateSeparator = makeSeparator()

        addLayoutGuide(bottomAnchorGuide)

        for v in [chipScroll, chipSeparator, candidateScroll, candidateSeparator] as [UIView] {
            addSubview(v)
        }

        NSLayoutConstraint.activate([
            // Chip scroll view
            chipScroll.topAnchor.constraint(equalTo: topAnchor, constant: 4),
            chipScroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            chipScroll.trailingAnchor.constraint(equalTo: trailingAnchor),
            chipScroll.heightAnchor.constraint(equalToConstant: 44),

            // Chip stack inside chip scroll
            chipStack.topAnchor.constraint(equalTo: chipScroll.topAnchor),
            chipStack.leadingAnchor.constraint(equalTo: chipScroll.leadingAnchor, constant: 8),
            chipStack.trailingAnchor.constraint(equalTo: chipScroll.trailingAnchor, constant: -8),
            chipStack.bottomAnchor.constraint(equalTo: chipScroll.bottomAnchor),
            chipStack.heightAnchor.constraint(equalTo: chipScroll.heightAnchor),

            chipSeparator.topAnchor.constraint(equalTo: chipScroll.bottomAnchor),
            chipSeparator.leadingAnchor.constraint(equalTo: leadingAnchor),
            chipSeparator.trailingAnchor.constraint(equalTo: trailingAnchor),
            chipSeparator.heightAnchor.constraint(equalToConstant: 0.5),

            // Candidate scroll view
            candidateScroll.topAnchor.constraint(equalTo: chipSeparator.bottomAnchor),
            candidateScroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            candidateScroll.trailingAnchor.constraint(equalTo: trailingAnchor),
            candidateScroll.heightAnchor.constraint(equalToConstant: 52),

            // Candidate stack inside candidate scroll
            candidateStack.topAnchor.constraint(equalTo: candidateScroll.topAnchor),
            candidateStack.leadingAnchor.constraint(equalTo: candidateScroll.leadingAnchor, constant: 8),
            candidateStack.trailingAnchor.constraint(equalTo: candidateScroll.trailingAnchor, constant: -8),
            candidateStack.bottomAnchor.constraint(equalTo: candidateScroll.bottomAnchor),
            candidateStack.heightAnchor.constraint(equalTo: candidateScroll.heightAnchor),

            candidateSeparator.topAnchor.constraint(equalTo: candidateScroll.bottomAnchor),
            candidateSeparator.leadingAnchor.constraint(equalTo: leadingAnchor),
            candidateSeparator.trailingAnchor.constraint(equalTo: trailingAnchor),
            candidateSeparator.heightAnchor.constraint(equalToConstant: 0.5),

            // Guide where the VC places the bottom row
            bottomAnchorGuide.topAnchor.constraint(equalTo: candidateSeparator.bottomAnchor),
            bottomAnchorGuide.leadingAnchor.constraint(equalTo: leadingAnchor),
            bottomAnchorGuide.trailingAnchor.constraint(equalTo: trailingAnchor),

            // Pin the view's own bottom so hitTest covers the chip + candidate rows.
            // Without this, the frame height is 0: content paints (clipsToBounds=false)
            // but all touches fall outside bounds and are never delivered to subviews.
            bottomAnchor.constraint(equalTo: candidateSeparator.bottomAnchor),
        ])
    }

    // MARK: - Public API

    // Rebuild the chip and candidate rows from the latest render state.
    // Call this every time the keyboard is in the panel state and a new
    // render state arrives from the session.
    func render(_ state: IosRenderState) {
        rebuildChips(state.segments)
        let selectedIdx = state.selectedIndex.map { Int($0) } ?? 0
        rebuildCandidates(state.candidates, selectedIndex: selectedIdx)
    }

    // MARK: - Builders

    private func rebuildChips(_ segments: [IosSegmentEntry]) {
        chipStack.arrangedSubviews.forEach { $0.removeFromSuperview() }

        let dismissBtn = makeSpecialButton("⊞", action: #selector(dismissTapped))
        chipStack.addArrangedSubview(dismissBtn)

        for (i, seg) in segments.enumerated() {
            chipStack.addArrangedSubview(makeChipContainer(text: seg.output, focused: seg.focused, index: i))
        }
    }

    private func rebuildCandidates(_ candidates: [String], selectedIndex: Int) {
        candidateStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        for (i, text) in candidates.enumerated() {
            candidateStack.addArrangedSubview(makeCandidateButton(text: text, index: i, selected: i == selectedIndex))
        }
    }

    // MARK: - Button factories

    // A chip is a container with the Khmer output label and a small ✏ edit button.
    // The ✏ button is only visible when the chip is focused so it doesn't clutter
    // unfocused chips.
    private func makeChipContainer(text: String, focused: Bool, index: Int) -> UIView {
        let container = UIView()
        container.translatesAutoresizingMaskIntoConstraints = false

        let chip = UIButton(type: .system)
        chip.setTitle(text, for: .normal)
        chip.titleLabel?.font = .systemFont(ofSize: 16, weight: focused ? .semibold : .regular)
        chip.setTitleColor(focused ? .systemBlue : .label, for: .normal)
        chip.backgroundColor = focused ? UIColor.systemBlue.withAlphaComponent(0.12) : UIColor.systemGray5
        chip.layer.cornerRadius = 12
        chip.contentEdgeInsets = UIEdgeInsets(top: 6, left: 12, bottom: 6, right: 12)
        chip.tag = index
        chip.addTarget(self, action: #selector(chipTapped(_:)), for: .touchUpInside)
        chip.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(chip)

        let editBtn = UIButton(type: .system)
        editBtn.setTitle("✏", for: .normal)
        editBtn.titleLabel?.font = .systemFont(ofSize: 12)
        editBtn.setTitleColor(.systemBlue, for: .normal)
        editBtn.isHidden = !focused
        editBtn.tag = index
        editBtn.addTarget(self, action: #selector(editTapped(_:)), for: .touchUpInside)
        editBtn.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(editBtn)

        NSLayoutConstraint.activate([
            chip.topAnchor.constraint(equalTo: container.topAnchor),
            chip.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            chip.bottomAnchor.constraint(equalTo: container.bottomAnchor),

            editBtn.leadingAnchor.constraint(equalTo: chip.trailingAnchor, constant: 2),
            editBtn.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            editBtn.centerYAnchor.constraint(equalTo: container.centerYAnchor),
            editBtn.widthAnchor.constraint(equalToConstant: 20),
        ])

        return container
    }

    private func makeCandidateButton(text: String, index: Int, selected: Bool) -> UIButton {
        let btn = UIButton(type: .system)
        btn.setTitle(text, for: .normal)
        btn.titleLabel?.font = .systemFont(ofSize: 20, weight: selected ? .semibold : .medium)
        btn.setTitleColor(selected ? .systemBlue : .label, for: .normal)
        btn.backgroundColor = selected ? UIColor.systemBlue.withAlphaComponent(0.08) : .white
        btn.layer.cornerRadius = 8
        btn.layer.borderWidth = selected ? 1.5 : 0
        btn.layer.borderColor = selected ? UIColor.systemBlue.cgColor : UIColor.clear.cgColor
        btn.contentEdgeInsets = UIEdgeInsets(top: 8, left: 14, bottom: 8, right: 14)
        btn.tag = index
        btn.addTarget(self, action: #selector(candidateTapped(_:)), for: .touchUpInside)
        return btn
    }

    private func makeSpecialButton(_ title: String, action: Selector) -> UIButton {
        let btn = UIButton(type: .system)
        btn.setTitle(title, for: .normal)
        btn.titleLabel?.font = .systemFont(ofSize: 15, weight: .medium)
        btn.setTitleColor(.black, for: .normal)
        KeyStyle.applySpecial(btn)
        btn.addTarget(self, action: action, for: .touchUpInside)
        return btn
    }

    private func makeSeparator() -> UIView {
        let v = UIView()
        v.backgroundColor = UIColor.separator
        v.translatesAutoresizingMaskIntoConstraints = false
        return v
    }

    // MARK: - Actions

    @objc private func dismissTapped()              { delegate?.candidatePanelDidDismiss(self) }
    @objc private func chipTapped(_ s: UIButton)    { delegate?.candidatePanel(self, didTapChipAt: s.tag) }
    @objc private func editTapped(_ s: UIButton)    { delegate?.candidatePanel(self, didRequestEditAt: s.tag) }
    @objc private func candidateTapped(_ s: UIButton) { delegate?.candidatePanel(self, didSelectCandidateAt: s.tag) }
}
