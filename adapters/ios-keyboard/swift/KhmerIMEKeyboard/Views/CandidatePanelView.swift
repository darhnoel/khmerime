import UIKit

// MARK: - Delegate

protocol CandidatePanelDelegate: AnyObject {
    /// User tapped a segment chip to move focus to that segment.
    func candidatePanel(_ panel: CandidatePanelView, didTapChipAt index: Int)
    /// User tapped ✏ on a chip to enter Segment Edit Mode for that segment.
    func candidatePanel(_ panel: CandidatePanelView, didRequestEditAt index: Int)
    /// User tapped a candidate to select it for the focused segment.
    func candidatePanel(_ panel: CandidatePanelView, didSelectCandidateAt index: Int)
    /// User tapped ✦ to dismiss the panel and return to the QWERTY view.
    func candidatePanelDidDismiss(_ panel: CandidatePanelView)
    /// User tapped the CharPick entry button (ក) in the word candidate panel.
    func candidatePanelDidEnterCharPick(_ panel: CandidatePanelView)
    /// User tapped a letter chip in CharPick mode.
    func candidatePanel(_ panel: CandidatePanelView, didTapCharPickLetter letter: Character)
}

// MARK: - Cell

private final class CandidateCell: UICollectionViewCell {
    static let reuseID = "CandidateCell"

    private let label = UILabel()

    override init(frame: CGRect) {
        super.init(frame: frame)
        // Required for self-sizing: contentView must use Auto Layout, not its
        // autoresizing mask, so systemLayoutSizeFitting returns the label's size.
        contentView.translatesAutoresizingMaskIntoConstraints = false
        label.font = .systemFont(ofSize: 20, weight: .medium)
        label.textAlignment = .center
        label.translatesAutoresizingMaskIntoConstraints = false
        contentView.addSubview(label)
        contentView.layer.cornerRadius = 8
        NSLayoutConstraint.activate([
            // Pin contentView to cell edges so the cell frame matches.
            contentView.topAnchor.constraint(equalTo: topAnchor),
            contentView.leadingAnchor.constraint(equalTo: leadingAnchor),
            contentView.trailingAnchor.constraint(equalTo: trailingAnchor),
            contentView.bottomAnchor.constraint(equalTo: bottomAnchor),

            label.topAnchor.constraint(equalTo: contentView.topAnchor, constant: 8),
            label.bottomAnchor.constraint(equalTo: contentView.bottomAnchor, constant: -8),
            label.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 14),
            label.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -14),
        ])
    }

    required init?(coder: NSCoder) { fatalError("use init(frame:)") }

    func configure(text: String, selected: Bool) {
        label.text = text
        label.font = .systemFont(ofSize: 20, weight: selected ? .semibold : .medium)
        label.textColor = .label
        let isDark = traitCollection.userInterfaceStyle == .dark
        contentView.backgroundColor = selected
            ? GlassColorSpec.selectedCandidateBackground(isDark: isDark)
            : GlassColorSpec.backgroundColor(isDark: isDark)
        contentView.layer.borderWidth = GlassColorSpec.candidateBorderWidth()
        contentView.layer.borderColor = GlassColorSpec.borderColor(isDark: isDark).cgColor
    }
}

// MARK: - View

// CandidatePanelView
// ==================
// Full-replacement view for the key-rows area, shown when the user taps ✦.
// The strip (StripView) remains above it at all times.
//
// Layout (replaces key rows; strip stays above):
//   ┌───────────────────────────────────────────┐
//   │  [✦]  [ណុំ ✏]  [ទៅ ✏]  [សាលារៀន ✏]  44pt │  ← chips (scrollable h)
//   ├───────────────────────────────────────────┤
//   │  ខ្ញុំ   ញុំ   ណុំ                        │
//   │  ណ៉ំ    ណ     …           adaptive height │  ← candidates (wrapped, scrollable v)
//   ├───────────────────────────────────────────┤
//   │  123  │      space      │  .  │    ⏎     │  ← bottom row (from VC)
//   └───────────────────────────────────────────┘

final class CandidatePanelView: UIView, KeyboardPanelDisplaying {

    weak var delegate: CandidatePanelDelegate?

    private let metrics: KeyboardLayoutMetrics

    // Exposed so the ViewController can embed the shared bottom row.
    let bottomAnchorGuide = UILayoutGuide()

    private let chipScroll  = UIScrollView()
    private let chipStack   = UIStackView()

    private let candidateCollection: UICollectionView = {
        let layout = UICollectionViewFlowLayout()
        layout.scrollDirection = .vertical
        // Concrete estimate so the layout has a valid starting size before
        // preferredLayoutAttributesFitting refines it. .automaticSize (-1,-1)
        // causes cells to size to zero on some iOS versions.
        layout.estimatedItemSize = CGSize(width: 64, height: 44)
        layout.minimumInteritemSpacing = 6
        layout.minimumLineSpacing = 6
        layout.sectionInset = UIEdgeInsets(top: 6, left: 8, bottom: 6, right: 8)
        let cv = UICollectionView(frame: .zero, collectionViewLayout: layout)
        cv.backgroundColor = .clear
        cv.showsVerticalScrollIndicator = true
        cv.showsHorizontalScrollIndicator = false
        cv.translatesAutoresizingMaskIntoConstraints = false
        return cv
    }()

    private var displayCandidates: [String] = []
    private var displaySelectedIndex: Int = 0

    init(metrics: KeyboardLayoutMetrics, frame: CGRect = .zero) {
        self.metrics = metrics
        super.init(frame: frame)
        setup()
    }

    override init(frame: CGRect) {
        self.metrics = KeyboardLayoutMetrics(device: .phone)
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) { fatalError("use init(frame:)") }

    // MARK: - Setup

    private func setup() {
        // Chip scroll
        chipScroll.showsHorizontalScrollIndicator = false
        chipScroll.backgroundColor = .clear
        chipScroll.translatesAutoresizingMaskIntoConstraints = false

        chipStack.axis = .horizontal
        chipStack.spacing = 8
        chipStack.alignment = .center
        chipStack.translatesAutoresizingMaskIntoConstraints = false
        chipScroll.addSubview(chipStack)

        // Chip stack uses contentLayoutGuide so chips can scroll past the visible edge.
        NSLayoutConstraint.activate([
            chipStack.topAnchor.constraint(equalTo: chipScroll.contentLayoutGuide.topAnchor),
            chipStack.leadingAnchor.constraint(equalTo: chipScroll.contentLayoutGuide.leadingAnchor, constant: 8),
            chipStack.trailingAnchor.constraint(equalTo: chipScroll.contentLayoutGuide.trailingAnchor, constant: -8),
            chipStack.bottomAnchor.constraint(equalTo: chipScroll.contentLayoutGuide.bottomAnchor),
            chipStack.heightAnchor.constraint(equalTo: chipScroll.frameLayoutGuide.heightAnchor),
        ])

        let chipSeparator = makeSeparator()

        // Candidate collection
        candidateCollection.dataSource = self
        candidateCollection.delegate   = self
        candidateCollection.register(CandidateCell.self, forCellWithReuseIdentifier: CandidateCell.reuseID)

        let candidateSeparator = makeSeparator()

        addLayoutGuide(bottomAnchorGuide)

        for v in [chipScroll, chipSeparator, candidateCollection, candidateSeparator] as [UIView] {
            addSubview(v)
        }

        NSLayoutConstraint.activate([
            // Chip scroll
            chipScroll.topAnchor.constraint(equalTo: topAnchor, constant: 4),
            chipScroll.leadingAnchor.constraint(equalTo: leadingAnchor),
            chipScroll.trailingAnchor.constraint(equalTo: trailingAnchor),
            chipScroll.heightAnchor.constraint(equalToConstant: metrics.panelChipHeight),

            chipSeparator.topAnchor.constraint(equalTo: chipScroll.bottomAnchor),
            chipSeparator.leadingAnchor.constraint(equalTo: leadingAnchor),
            chipSeparator.trailingAnchor.constraint(equalTo: trailingAnchor),
            chipSeparator.heightAnchor.constraint(equalToConstant: 0.5),

            // Candidate collection — wraps and scrolls vertically
            candidateCollection.topAnchor.constraint(equalTo: chipSeparator.bottomAnchor),
            candidateCollection.leadingAnchor.constraint(equalTo: leadingAnchor),
            candidateCollection.trailingAnchor.constraint(equalTo: trailingAnchor),
            candidateCollection.heightAnchor.constraint(equalToConstant: metrics.panelCandidateHeight),

            candidateSeparator.topAnchor.constraint(equalTo: candidateCollection.bottomAnchor),
            candidateSeparator.leadingAnchor.constraint(equalTo: leadingAnchor),
            candidateSeparator.trailingAnchor.constraint(equalTo: trailingAnchor),
            candidateSeparator.heightAnchor.constraint(equalToConstant: 0.5),

            // Guide where the VC places the bottom row
            bottomAnchorGuide.topAnchor.constraint(equalTo: candidateSeparator.bottomAnchor),
            bottomAnchorGuide.leadingAnchor.constraint(equalTo: leadingAnchor),
            bottomAnchorGuide.trailingAnchor.constraint(equalTo: trailingAnchor),

            // Pin view's own bottom so hit-testing works on all subviews.
            bottomAnchor.constraint(equalTo: candidateSeparator.bottomAnchor),
        ])
    }

    // MARK: - Public API

    func render(_ state: IosRenderState) {
        rebuildChips(state.segments)
        let selectedIdx = state.selectedIndex.map { Int($0) } ?? 0
        rebuildCandidates(state.candidates, selectedIndex: selectedIdx)
    }

    /// Update only the candidate collection without touching the chip row.
    /// Used in CharPick mode where the alphabet chips must stay intact.
    func renderCharPickCandidates(_ candidates: [String]) {
        rebuildCandidates(candidates, selectedIndex: 0)
    }

    // Letters that have at least one Khmer mapping in khmer_character_relation.csv.
    // f, q, w, x, z have no mappings and are omitted.
    private static let charPickLetters = "abcdeghijklmnoprstuvy"

    /// Switch the chip row to the mapped-letter picker used in CharPick mode.
    /// Clears any displayed candidates.
    func renderCharPickAlphabet() {
        chipStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        chipStack.addArrangedSubview(makeSpecialButton("✦", action: #selector(dismissTapped)))
        for letter in Self.charPickLetters {
            chipStack.addArrangedSubview(makeLetterChip(String(letter).uppercased()))
        }
        displayCandidates = []
        displaySelectedIndex = 0
        candidateCollection.reloadData()
    }

    // MARK: - Builders

    private func rebuildChips(_ segments: [IosSegmentEntry]) {
        chipStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        chipStack.addArrangedSubview(makeSpecialButton("✦", action: #selector(dismissTapped)))
        // CharPick entry button — lets the user discard the current composition
        // and switch to individual character picking.
        chipStack.addArrangedSubview(makeSpecialButton("ក…", action: #selector(enterCharPickTapped)))
        for (i, seg) in segments.enumerated() {
            chipStack.addArrangedSubview(makeChipContainer(text: seg.output, focused: seg.focused, index: i))
        }
    }

    private func rebuildCandidates(_ candidates: [String], selectedIndex: Int) {
        displayCandidates = candidates
        displaySelectedIndex = selectedIndex
        candidateCollection.reloadData()
        if !candidates.isEmpty {
            candidateCollection.scrollToItem(at: IndexPath(item: 0, section: 0), at: .top, animated: false)
        }
    }

    // MARK: - Button factories

    private func makeChipContainer(text: String, focused: Bool, index: Int) -> UIView {
        let container = UIView()
        container.translatesAutoresizingMaskIntoConstraints = false

        var chipConfig = UIButton.Configuration.plain()
        chipConfig.title = text
        chipConfig.baseForegroundColor = .label
        chipConfig.contentInsets = NSDirectionalEdgeInsets(top: 6, leading: 12, bottom: 6, trailing: 12)
        chipConfig.titleTextAttributesTransformer = UIConfigurationTextAttributesTransformer { attrs in
            var a = attrs
            a.font = UIFont.systemFont(ofSize: 16, weight: focused ? .semibold : .regular)
            return a
        }
        let isDark = UITraitCollection.current.userInterfaceStyle == .dark
        chipConfig.background.backgroundColor = focused
            ? GlassColorSpec.selectedCandidateBackground(isDark: isDark)
            : GlassColorSpec.backgroundColor(isDark: isDark)
        chipConfig.background.cornerRadius = 12
        chipConfig.background.strokeWidth = GlassColorSpec.candidateBorderWidth()
        chipConfig.background.strokeColor = GlassColorSpec.borderColor(isDark: isDark)
        let chip = UIButton(configuration: chipConfig)
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

    private func makeSpecialButton(_ title: String, action: Selector) -> UIButton {
        let btn = GlassKeyButton(frame: .zero)
        btn.setTitle(title, for: .normal)
        btn.titleLabel?.font = .systemFont(ofSize: 15, weight: .medium)
        KeyStyle.applySpecial(btn, isActive: title == "✦")
        btn.addTarget(self, action: action, for: .touchUpInside)
        return btn
    }

    private func makeSeparator() -> UIView {
        let v = UIView()
        v.backgroundColor = UIColor.separator
        v.translatesAutoresizingMaskIntoConstraints = false
        return v
    }

    // MARK: - Button factories (CharPick alphabet)

    private func makeLetterChip(_ letter: String) -> UIButton {
        let btn = GlassKeyButton(frame: .zero)
        btn.setTitle(letter, for: .normal)
        KeyStyle.applyLetter(btn)
        btn.titleLabel?.font = .systemFont(ofSize: 16, weight: .medium)
        btn.contentEdgeInsets = UIEdgeInsets(top: 6, left: 10, bottom: 6, right: 10)
        // UITapGestureRecognizer is not blocked by the parent UIScrollView's
        // delaysContentTouches / canCancelContentTouches settings, so it fires
        // reliably while still allowing the scroll view to scroll.
        let tap = UITapGestureRecognizer(target: self, action: #selector(charPickLetterTapped(_:)))
        btn.addGestureRecognizer(tap)
        return btn
    }

    // MARK: - Actions

    @objc private func dismissTapped()           { delegate?.candidatePanelDidDismiss(self) }
    @objc private func enterCharPickTapped()     { delegate?.candidatePanelDidEnterCharPick(self) }
    @objc private func chipTapped(_ s: UIButton) { delegate?.candidatePanel(self, didTapChipAt: s.tag) }
    @objc private func editTapped(_ s: UIButton) { delegate?.candidatePanel(self, didRequestEditAt: s.tag) }

    @objc private func charPickLetterTapped(_ sender: UITapGestureRecognizer) {
        guard let btn = sender.view as? UIButton,
              let title = btn.title(for: .normal),
              let letter = title.lowercased().first else { return }
        delegate?.candidatePanel(self, didTapCharPickLetter: letter)
    }
}

// MARK: - UICollectionViewDataSource / Delegate

extension CandidatePanelView: UICollectionViewDataSource, UICollectionViewDelegate {

    func collectionView(_ collectionView: UICollectionView, numberOfItemsInSection section: Int) -> Int {
        displayCandidates.count
    }

    func collectionView(_ collectionView: UICollectionView, cellForItemAt indexPath: IndexPath) -> UICollectionViewCell {
        let cell = collectionView.dequeueReusableCell(
            withReuseIdentifier: CandidateCell.reuseID, for: indexPath) as! CandidateCell
        cell.configure(text: displayCandidates[indexPath.item], selected: indexPath.item == displaySelectedIndex)
        return cell
    }

    func collectionView(_ collectionView: UICollectionView, didSelectItemAt indexPath: IndexPath) {
        delegate?.candidatePanel(self, didSelectCandidateAt: indexPath.item)
    }
}
