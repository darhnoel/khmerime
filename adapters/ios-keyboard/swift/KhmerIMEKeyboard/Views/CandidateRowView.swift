import UIKit

protocol KeyboardCandidateRowDisplaying: AnyObject {
    func render(_ state: IosRenderState)
    func clear()
}

// CandidateRowView
// ================
// Persistent row of candidate chips for the focused segment, shown above the
// qwerty keys at all times (replacing the old toggle-into-a-panel browsing
// flow). Hidden only while CharPick is active.
final class CandidateRowView: UIView, KeyboardCandidateRowDisplaying {

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

        stack.axis = .horizontal
        stack.spacing = 8
        stack.alignment = .center
        stack.translatesAutoresizingMaskIntoConstraints = false
        addSubview(stack)

        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            stack.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -8),
            stack.topAnchor.constraint(equalTo: topAnchor),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor),
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
    }

    // MARK: - Tap

    // Pure-ish hit test, separated from the @objc gesture glue so it can be
    // driven directly from tests without simulating a real touch.
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
