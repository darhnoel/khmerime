import UIKit

final class StripView: UIView, KeyboardStripDisplaying {

    private let romanRow = UILabel()
    private let khmerRow = UIStackView()
    private let segmentPool = StripLabelPool()
    private var tappableSegmentLabels: [UILabel] = []

    var onKhmerRowTapped: (() -> Void)?
    var onKhmerRowLongPressed: (() -> Void)?
    var onSegmentFocused: ((Int) -> Void)?

    // Pure hit-test: which label (if any) contains `point`. A single tap
    // recognizer on the whole row delegates to this instead of racing two
    // recognizers (one on the row, one per label) for the same touch.
    static func segmentIndex(at point: CGPoint, labelFrames: [CGRect]) -> Int? {
        labelFrames.firstIndex { $0.contains(point) }
    }

    override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) { fatalError("use init(frame:)") }

    // MARK: - Public API

    func render(_ state: IosRenderState, romanBuffer: String) {
        romanRow.text = StripPresentationSpec.romanRowText(state: state, romanBuffer: romanBuffer)
        rebuildKhmerRow(state: state)
    }

    func clear() {
        romanRow.text = ""
        segmentPool.sync(count: 0, in: khmerRow)
    }

    // MARK: - Setup

    private func setup() {
        backgroundColor = .clear

        romanRow.font = .systemFont(ofSize: 12)
        romanRow.textColor = .secondaryLabel
        romanRow.textAlignment = .center
        romanRow.translatesAutoresizingMaskIntoConstraints = false
        addSubview(romanRow)

        khmerRow.axis = .horizontal
        khmerRow.spacing = 8
        khmerRow.alignment = .center
        khmerRow.distribution = .equalSpacing
        khmerRow.translatesAutoresizingMaskIntoConstraints = false
        addSubview(khmerRow)

        let separator = UIView()
        separator.backgroundColor = UIColor.separator
        separator.translatesAutoresizingMaskIntoConstraints = false
        addSubview(separator)

        NSLayoutConstraint.activate([
            romanRow.topAnchor.constraint(equalTo: topAnchor, constant: 2),
            romanRow.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            romanRow.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            romanRow.heightAnchor.constraint(equalToConstant: 18),

            khmerRow.topAnchor.constraint(equalTo: romanRow.bottomAnchor, constant: 2),
            khmerRow.centerXAnchor.constraint(equalTo: centerXAnchor),
            khmerRow.bottomAnchor.constraint(equalTo: separator.topAnchor, constant: -2),

            separator.leadingAnchor.constraint(equalTo: leadingAnchor),
            separator.trailingAnchor.constraint(equalTo: trailingAnchor),
            separator.bottomAnchor.constraint(equalTo: bottomAnchor),
            separator.heightAnchor.constraint(equalToConstant: 0.5),
        ])

        addKhmerRowTapGesture()
    }

    // MARK: - Khmer row chips

    private func rebuildKhmerRow(state: IosRenderState) {
        let texts = StripPresentationSpec.segmentKhmerTexts(state: state)
        let focusedIdx = StripPresentationSpec.focusedSegmentIndex(state: state)

        if texts.isEmpty {
            let candidate = state.candidates.isEmpty ? "" :
                state.candidates[state.selectedIndex.map { Int($0) } ?? 0]
            guard !candidate.isEmpty else {
                segmentPool.sync(count: 0, in: khmerRow)
                tappableSegmentLabels = []
                return
            }
            let visible = segmentPool.sync(count: 1, in: khmerRow)
            let lbl = visible[0]
            lbl.attributedText = nil
            lbl.text = candidate
            lbl.font = .systemFont(ofSize: 18, weight: .medium)
            lbl.textColor = .label
            tappableSegmentLabels = visible
        } else {
            let visible = segmentPool.sync(count: texts.count, in: khmerRow)
            for (idx, lbl) in visible.enumerated() {
                let text = texts[idx]
                let focused = idx == focusedIdx
                if focused {
                    lbl.attributedText = NSAttributedString(string: text, attributes: [
                        .font: UIFont.systemFont(ofSize: 18, weight: .bold),
                        .foregroundColor: UIColor.label,
                        .underlineStyle: NSUnderlineStyle.single.rawValue,
                    ])
                } else {
                    lbl.attributedText = nil
                    lbl.text = text
                    lbl.font = .systemFont(ofSize: 18)
                    lbl.textColor = .secondaryLabel
                }
            }
            tappableSegmentLabels = visible
        }
    }

    // MARK: - Tap / long-press on khmer row

    private func addKhmerRowTapGesture() {
        let tap = UITapGestureRecognizer(target: self, action: #selector(khmerRowTapped(_:)))
        khmerRow.addGestureRecognizer(tap)
        let longPress = UILongPressGestureRecognizer(target: self, action: #selector(khmerRowLongPressed))
        longPress.minimumPressDuration = 0.4
        khmerRow.addGestureRecognizer(longPress)
    }

    // A single recognizer on the whole row decides, per tap, whether it landed
    // on a chip (→ focus that segment) or empty row space (→ commit). Avoids
    // running two recognizers (row + per-label) that would otherwise both
    // fire for the same touch.
    @objc private func khmerRowTapped(_ gr: UITapGestureRecognizer) {
        let point = gr.location(in: khmerRow)
        let frames = tappableSegmentLabels.map { $0.frame }
        if let index = StripView.segmentIndex(at: point, labelFrames: frames) {
            onSegmentFocused?(index)
        } else {
            onKhmerRowTapped?()
        }
    }

    @objc private func khmerRowLongPressed(_ gr: UILongPressGestureRecognizer) {
        guard gr.state == .began else { return }
        onKhmerRowLongPressed?()
    }
}
