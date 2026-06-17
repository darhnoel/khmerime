import UIKit

protocol KeyboardCandidateRowDisplaying: AnyObject {
    func render(_ state: IosRenderState)
    func clear()
}

final class CandidateRowView: UIView, KeyboardCandidateRowDisplaying {

    private let scrollView = UIScrollView()
    private let stack = UIStackView()
    private let pool = StripLabelPool()
    private var tappableLabels: [UILabel] = []

    var onCandidateSelected: ((Int) -> Void)?

    override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) { fatalError("use init(frame:)") }

    // MARK: - Public API

    func render(_ state: IosRenderState) {
        let selectedIndex = state.selectedIndex.map { Int($0) } ?? 0
        rebuild(candidates: state.candidates, selectedIndex: selectedIndex)
    }

    func clear() {
        rebuild(candidates: [], selectedIndex: 0)
    }

    // MARK: - Setup

    private func setup() {
        backgroundColor = .clear

        scrollView.showsHorizontalScrollIndicator = false
        scrollView.showsVerticalScrollIndicator = false
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(scrollView)

        stack.axis = .horizontal
        stack.spacing = 8
        stack.alignment = .center
        stack.translatesAutoresizingMaskIntoConstraints = false
        scrollView.addSubview(stack)

        NSLayoutConstraint.activate([
            scrollView.leadingAnchor.constraint(equalTo: leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: trailingAnchor),
            scrollView.topAnchor.constraint(equalTo: topAnchor),
            scrollView.bottomAnchor.constraint(equalTo: bottomAnchor),

            stack.leadingAnchor.constraint(equalTo: scrollView.contentLayoutGuide.leadingAnchor, constant: 8),
            stack.trailingAnchor.constraint(equalTo: scrollView.contentLayoutGuide.trailingAnchor, constant: -8),
            stack.topAnchor.constraint(equalTo: scrollView.contentLayoutGuide.topAnchor),
            stack.bottomAnchor.constraint(equalTo: scrollView.contentLayoutGuide.bottomAnchor),
            stack.heightAnchor.constraint(equalTo: scrollView.frameLayoutGuide.heightAnchor),
        ])

        let tap = UITapGestureRecognizer(target: self, action: #selector(rowTapped(_:)))
        stack.addGestureRecognizer(tap)
    }

    // MARK: - Chips

    private func rebuild(candidates: [String], selectedIndex: Int) {
        let visible = pool.sync(count: candidates.count, in: stack)
        for (idx, label) in visible.enumerated() {
            label.text = candidates[idx]
            let selected = idx == selectedIndex
            label.font = .systemFont(ofSize: 18, weight: selected ? .semibold : .regular)
            label.textColor = selected ? .label : .secondaryLabel
        }
        tappableLabels = visible
        scrollSelectedIntoView(selectedIndex: selectedIndex, in: visible)
    }

    private func scrollSelectedIntoView(selectedIndex: Int, in labels: [UILabel]) {
        guard labels.indices.contains(selectedIndex) else { return }
        // Deferred so Auto Layout has resolved the label frame first.
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            let label = labels[selectedIndex]
            let rect = self.scrollView.convert(label.frame, from: self.stack)
            self.scrollView.scrollRectToVisible(rect, animated: false)
        }
    }

    // MARK: - Tap

    func handleTap(at point: CGPoint, in coordinateSpace: UIView) {
        let pointInStack = stack.convert(point, from: coordinateSpace)
        let frames = tappableLabels.map { $0.frame }
        if let index = StripView.segmentIndex(at: pointInStack, labelFrames: frames) {
            onCandidateSelected?(index)
        }
    }

    @objc private func rowTapped(_ gr: UITapGestureRecognizer) {
        handleTap(at: gr.location(in: stack), in: stack)
    }
}
