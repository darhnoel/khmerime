import UIKit

// PhraseWheelView
// ===============
// The default mobile candidate surface (ADR-0014): a horizontal, center-snapped
// carousel of whole-phrase hypotheses. One card per Phrase Candidate, in rank
// order (raw roman last). Snapping + selection reporting build on the pure
// `PhraseWheelLayout` math. Conforms to KeyboardCandidateRowDisplaying so it can
// occupy the candidate-row slot in the keyboard hierarchy.

final class PhraseWheelView: UIView, KeyboardCandidateRowDisplaying {

    private let scrollView = UIScrollView()
    private let stack = UIStackView()
    private let pool = StripLabelPool()

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
        let visible = pool.sync(count: texts.count, in: stack)
        for (index, label) in visible.enumerated() {
            label.text = texts[index]
            label.font = .systemFont(ofSize: 20, weight: .regular)
            label.textColor = .label
        }
    }
}
