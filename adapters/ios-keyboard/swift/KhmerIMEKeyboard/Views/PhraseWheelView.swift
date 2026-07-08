import UIKit

// PhraseWheelView
// ===============
// The default mobile candidate surface (ADR-0014): a horizontal, center-snapped
// carousel of whole-phrase hypotheses. One card per Phrase Candidate, in rank
// order (raw roman last). The card nearest the view's horizontal center is the
// selection; settling reports it via `onPhraseSelected` (→ session.selectPhrase)
// and highlights it. Snapping builds on the pure `PhraseWheelLayout` math.
// Conforms to KeyboardCandidateRowDisplaying so it occupies the candidate-row slot.

final class PhraseWheelView: UIView, KeyboardCandidateRowDisplaying {

    private let scrollView = UIScrollView()
    private let stack = UIStackView()
    private let pool = StripLabelPool()
    private var cards: [UILabel] = []
    private var selectedIndex = 0

    /// Called when the centered card changes as the user scrolls — the index of the
    /// now-selected Phrase Candidate. The controller forwards it to `selectPhrase`.
    var onPhraseSelected: ((Int) -> Void)?

    override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) { fatalError("use init(frame:)") }

    // MARK: - Public API

    func render(_ state: IosRenderState, presentation: CandidateRowPresentation = .composition) {
        rebuild(texts: state.phraseCandidates.map { $0.text })
    }

    func clear() {
        rebuild(texts: [])
    }

    // MARK: - Setup

    private func setup() {
        backgroundColor = .clear

        scrollView.showsHorizontalScrollIndicator = false
        scrollView.showsVerticalScrollIndicator = false
        scrollView.delegate = self
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
    }

    // MARK: - Cards

    private func rebuild(texts: [String]) {
        cards = pool.sync(count: texts.count, in: stack)
        for (index, label) in cards.enumerated() {
            label.text = texts[index]
        }
        selectedIndex = 0
        applyHighlight()
    }

    private func applyHighlight() {
        for (index, label) in cards.enumerated() {
            let selected = index == selectedIndex
            label.font = .systemFont(ofSize: 20, weight: selected ? .semibold : .regular)
            label.textColor = selected ? .label : .secondaryLabel
        }
    }

    // MARK: - Snap + selection

    private func cardCenters() -> [CGFloat] {
        cards.map { label in
            scrollView.convert(label.frame, from: label.superview).midX
        }
    }

    /// The content offset that centers card `index`. Exposed for the scroll wiring
    /// and for tests.
    func centerOffset(forCardIndex index: Int) -> CGFloat? {
        PhraseWheelLayout.centerOffset(forCardIndex: index, cardCenters: cardCenters(), viewWidth: bounds.width)
    }

    /// Given a resting horizontal scroll offset, snap to the nearest card, highlight
    /// it, and report the selection if it changed.
    func settleSelection(atContentOffsetX offsetX: CGFloat) {
        let centers = cardCenters()
        guard let index = PhraseWheelLayout.nearestCardIndex(toCenterX: offsetX + bounds.width / 2,
                                                             cardCenters: centers) else { return }
        if let target = PhraseWheelLayout.centerOffset(forCardIndex: index, cardCenters: centers,
                                                       viewWidth: bounds.width) {
            let clamped = max(0, target)
            if abs(scrollView.contentOffset.x - clamped) > 0.5 {
                scrollView.setContentOffset(CGPoint(x: clamped, y: 0), animated: true)
            }
        }
        guard index != selectedIndex else { return }
        selectedIndex = index
        applyHighlight()
        onPhraseSelected?(index)
    }
}

extension PhraseWheelView: UIScrollViewDelegate {
    func scrollViewDidEndDecelerating(_ scrollView: UIScrollView) {
        settleSelection(atContentOffsetX: scrollView.contentOffset.x)
    }

    func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
        if !decelerate { settleSelection(atContentOffsetX: scrollView.contentOffset.x) }
    }
}
