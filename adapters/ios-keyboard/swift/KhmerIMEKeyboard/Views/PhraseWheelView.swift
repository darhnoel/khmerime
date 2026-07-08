import UIKit

// PhraseWheelView (ADR-0015)
// ==========================
// A horizontal row of the *alternative* Phrase Candidates — the whole-phrase Khmer
// hypotheses other than the selected one, which the strip already shows. Centered
// when the cards fit, left-padded + horizontally scrollable when they overflow
// (reusing CandidateRowLayout). Tapping a card selects that phrase for preview.
// Conforms to KeyboardCandidateRowDisplaying so it occupies the candidate-row slot.

final class PhraseWheelView: UIView, KeyboardCandidateRowDisplaying {

    private let scrollView = UIScrollView()
    private let stack = UIStackView()
    private let pool = StripLabelPool()
    private var tappableLabels: [UILabel] = []

    // Left/right breathing room so cards never touch the screen edge when they overflow.
    private static let edgeInset: CGFloat = 16

    /// Tapping a card selects Phrase Candidate `index` — it becomes the strip's
    /// preview; Space/Enter then commit it. Tapping never commits (ADR-0015).
    var onPhraseSelected: ((Int) -> Void)?

    /// Whether there is anything to show (≥1 alternative). The surface hides the row
    /// entirely when false, so the strip stands alone.
    var hasAlternatives: Bool { !tappableLabels.isEmpty }

    override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) { fatalError("use init(frame:)") }

    // MARK: - Public API

    func render(_ state: IosRenderState, presentation: CandidateRowPresentation = .composition) {
        // Alternatives only — the strip owns the selected hypothesis.
        let selectedIndex = Int(state.selectedPhraseIndex)
        let alternatives = state.phraseCandidates.enumerated()
            .filter { index, _ in index != selectedIndex }
            .map { index, phrase in (index: index, text: phrase.text) }
        rebuild(alternatives: alternatives)
    }

    func clear() {
        rebuild(alternatives: [])
    }

    // MARK: - Setup

    private func setup() {
        backgroundColor = .clear

        scrollView.showsHorizontalScrollIndicator = false
        scrollView.showsVerticalScrollIndicator = false
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(scrollView)

        stack.axis = .horizontal
        stack.spacing = 16
        stack.alignment = .center
        stack.translatesAutoresizingMaskIntoConstraints = false
        scrollView.addSubview(stack)

        NSLayoutConstraint.activate([
            scrollView.leadingAnchor.constraint(equalTo: leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: trailingAnchor),
            scrollView.topAnchor.constraint(equalTo: topAnchor),
            scrollView.bottomAnchor.constraint(equalTo: bottomAnchor),

            stack.leadingAnchor.constraint(equalTo: scrollView.contentLayoutGuide.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: scrollView.contentLayoutGuide.trailingAnchor),
            stack.topAnchor.constraint(equalTo: scrollView.contentLayoutGuide.topAnchor),
            stack.bottomAnchor.constraint(equalTo: scrollView.contentLayoutGuide.bottomAnchor),
            stack.heightAnchor.constraint(equalTo: scrollView.frameLayoutGuide.heightAnchor),
        ])

        let tap = UITapGestureRecognizer(target: self, action: #selector(rowTapped(_:)))
        stack.addGestureRecognizer(tap)
    }

    // Center the cards while they all fit; once they overflow, fall back to the edge
    // inset so the row left-aligns (with left breathing room) and scrolls. Same math
    // as the candidate row.
    override func layoutSubviews() {
        super.layoutSubviews()
        let contentWidth = stack.systemLayoutSizeFitting(UIView.layoutFittingCompressedSize).width
        let inset = CandidateRowLayout.centeringInset(
            contentWidth: contentWidth,
            availableWidth: bounds.width,
            edgeInset: Self.edgeInset
        )
        if scrollView.contentInset.left != inset {
            scrollView.contentInset = UIEdgeInsets(top: 0, left: inset, bottom: 0, right: inset)
        }
    }

    // MARK: - Cards

    private func rebuild(alternatives: [(index: Int, text: String)]) {
        tappableLabels = pool.sync(count: alternatives.count, in: stack)
        for (index, label) in tappableLabels.enumerated() {
            label.text = alternatives[index].text
            label.tag = alternatives[index].index
            label.font = .systemFont(ofSize: 20, weight: .regular)
            label.textColor = .label
        }
        setNeedsLayout()
    }

    // MARK: - Tap → select

    @objc private func rowTapped(_ gr: UITapGestureRecognizer) {
        let point = gr.location(in: stack)
        let frames = tappableLabels.map { $0.frame }
        if let index = StripView.segmentIndex(at: point, labelFrames: frames) {
            onPhraseSelected?(tappableLabels[index].tag)
        }
    }
}
