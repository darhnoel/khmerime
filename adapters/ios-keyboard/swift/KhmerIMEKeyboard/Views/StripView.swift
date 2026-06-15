import UIKit

final class StripView: UIView, KeyboardStripDisplaying {

    private let romanRow = UILabel()
    private let khmerRow = UIStackView()

    var onKhmerRowTapped: (() -> Void)?
    var onKhmerRowLongPressed: (() -> Void)?
    var onSegmentFocused: ((Int) -> Void)?

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
        khmerRow.arrangedSubviews.forEach { $0.removeFromSuperview() }
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
        khmerRow.arrangedSubviews.forEach { $0.removeFromSuperview() }
        let texts = StripPresentationSpec.segmentKhmerTexts(state: state)
        let focusedIdx = StripPresentationSpec.focusedSegmentIndex(state: state)

        if texts.isEmpty {
            let candidate = state.candidates.isEmpty ? "" :
                state.candidates[state.selectedIndex.map { Int($0) } ?? 0]
            if !candidate.isEmpty {
                khmerRow.addArrangedSubview(makeCandidateLabel(candidate))
            }
        } else {
            for (idx, text) in texts.enumerated() {
                khmerRow.addArrangedSubview(makeSegmentLabel(text, index: idx, focused: idx == focusedIdx))
            }
        }
    }

    private func makeSegmentLabel(_ text: String, index: Int, focused: Bool) -> UILabel {
        let lbl = UILabel()
        lbl.text = text
        lbl.font = focused
            ? .systemFont(ofSize: 18, weight: .bold)
            : .systemFont(ofSize: 18)
        lbl.textColor = focused ? .label : .secondaryLabel
        if focused {
            lbl.attributedText = NSAttributedString(string: text, attributes: [
                .font: UIFont.systemFont(ofSize: 18, weight: .bold),
                .foregroundColor: UIColor.label,
                .underlineStyle: NSUnderlineStyle.single.rawValue,
            ])
        }
        lbl.isUserInteractionEnabled = true
        lbl.addGestureRecognizer(
            SegmentTapGestureRecognizer(index: index) { [weak self] idx in
                self?.onSegmentFocused?(idx)
            }
        )
        return lbl
    }

    private func makeCandidateLabel(_ text: String) -> UILabel {
        let lbl = UILabel()
        lbl.text = text
        lbl.font = .systemFont(ofSize: 18, weight: .medium)
        lbl.textColor = .label
        return lbl
    }

    // MARK: - Tap / long-press on khmer row (no segments)

    private func addKhmerRowTapGesture() {
        let tap = UITapGestureRecognizer(target: self, action: #selector(khmerRowTapped))
        khmerRow.addGestureRecognizer(tap)
        let longPress = UILongPressGestureRecognizer(target: self, action: #selector(khmerRowLongPressed))
        longPress.minimumPressDuration = 0.4
        khmerRow.addGestureRecognizer(longPress)
    }

    @objc private func khmerRowTapped()  { onKhmerRowTapped?() }

    @objc private func khmerRowLongPressed(_ gr: UILongPressGestureRecognizer) {
        guard gr.state == .began else { return }
        onKhmerRowLongPressed?()
    }
}

// Carries the segment index through the gesture recognizer.
private final class SegmentTapGestureRecognizer: UITapGestureRecognizer {
    private let index: Int
    private let handler: (Int) -> Void

    init(index: Int, handler: @escaping (Int) -> Void) {
        self.index = index
        self.handler = handler
        super.init(target: nil, action: nil)
        addTarget(self, action: #selector(fired))
    }

    @objc private func fired() { handler(index) }
}
