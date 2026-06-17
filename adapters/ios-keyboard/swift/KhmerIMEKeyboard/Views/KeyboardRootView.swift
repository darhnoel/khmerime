import UIKit

protocol KeyboardStripDisplaying: AnyObject {
    func render(_ state: IosRenderState, romanBuffer: String)
    func clear()
}

final class KeyboardRootView: UIView {
    private let stripDisplay: KeyboardStripDisplaying
    private let candidateRowDisplay: KeyboardCandidateRowDisplaying
    private let qwertyView: UIView
    private let numericView: UIView
    private let symbolsView: UIView
    private let candidateRowView: UIView

    init(
        metrics: KeyboardLayoutMetrics,
        stripView: UIView & KeyboardStripDisplaying,
        qwertyView: UIView,
        numericView: UIView,
        symbolsView: UIView,
        candidateRowView: UIView & KeyboardCandidateRowDisplaying
    ) {
        self.stripDisplay = stripView
        self.candidateRowDisplay = candidateRowView
        self.qwertyView = qwertyView
        self.numericView = numericView
        self.symbolsView = symbolsView
        self.candidateRowView = candidateRowView
        super.init(frame: .zero)

        backgroundColor = .clear
        for view in [stripView, candidateRowView, qwertyView, numericView, symbolsView] {
            view.translatesAutoresizingMaskIntoConstraints = false
            addSubview(view)
        }

        NSLayoutConstraint.activate([
            stripView.topAnchor.constraint(equalTo: topAnchor),
            stripView.leadingAnchor.constraint(equalTo: leadingAnchor),
            stripView.trailingAnchor.constraint(equalTo: trailingAnchor),
            stripView.heightAnchor.constraint(equalToConstant: metrics.stripHeight),

            candidateRowView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            candidateRowView.leadingAnchor.constraint(equalTo: leadingAnchor),
            candidateRowView.trailingAnchor.constraint(equalTo: trailingAnchor),
            candidateRowView.heightAnchor.constraint(equalToConstant: metrics.candidateRowHeight),

            qwertyView.topAnchor.constraint(equalTo: candidateRowView.bottomAnchor),
            qwertyView.leadingAnchor.constraint(equalTo: leadingAnchor),
            qwertyView.trailingAnchor.constraint(equalTo: trailingAnchor),
            qwertyView.bottomAnchor.constraint(equalTo: bottomAnchor),

            numericView.topAnchor.constraint(equalTo: candidateRowView.bottomAnchor),
            numericView.leadingAnchor.constraint(equalTo: leadingAnchor),
            numericView.trailingAnchor.constraint(equalTo: trailingAnchor),
            numericView.bottomAnchor.constraint(equalTo: bottomAnchor),

            symbolsView.topAnchor.constraint(equalTo: candidateRowView.bottomAnchor),
            symbolsView.leadingAnchor.constraint(equalTo: leadingAnchor),
            symbolsView.trailingAnchor.constraint(equalTo: trailingAnchor),
            symbolsView.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])

        apply(.qwerty)
    }

    required init?(coder: NSCoder) { fatalError("use init(metrics:stripView:qwertyView:numericView:symbolsView:candidateRowView:)") }

    func apply(_ state: KeyboardState) {
        let visibility = KeyboardLayerVisibility(state: state)
        qwertyView.isHidden = !visibility.showsQwerty
        numericView.isHidden = !visibility.showsNumeric
        symbolsView.isHidden = !visibility.showsSymbols
        candidateRowView.isHidden = !visibility.showsCandidateRow
    }

    func render(_ state: IosRenderState, romanHint: String) {
        stripDisplay.render(state, romanBuffer: romanHint)
        candidateRowDisplay.render(state)
    }

    func clearStrip() {
        stripDisplay.clear()
        candidateRowDisplay.clear()
    }
}
