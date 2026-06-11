import UIKit

protocol KeyboardStripDisplaying: AnyObject {
    func render(_ state: IosRenderState, romanBuffer: String)
    func clear()
}

protocol KeyboardPanelDisplaying: AnyObject {
    var bottomAnchorGuide: UILayoutGuide { get }
    func render(_ state: IosRenderState)
    func renderCharPickCandidates(_ candidates: [String])
    func renderCharPickAlphabet()
}

final class KeyboardRootView: UIView {
    private let stripDisplay: KeyboardStripDisplaying
    private let panelDisplay: KeyboardPanelDisplaying
    private let qwertyView: UIView
    private let numericView: UIView
    private let symbolsView: UIView
    private let panelView: UIView
    private let panelBottomRow: UIView

    init(
        metrics: KeyboardLayoutMetrics,
        stripView: UIView & KeyboardStripDisplaying,
        qwertyView: UIView,
        numericView: UIView,
        symbolsView: UIView,
        panelView: UIView & KeyboardPanelDisplaying,
        panelBottomRow: UIView,
        panelBottomAnchorGuide: UILayoutGuide
    ) {
        self.stripDisplay = stripView
        self.panelDisplay = panelView
        self.qwertyView = qwertyView
        self.numericView = numericView
        self.symbolsView = symbolsView
        self.panelView = panelView
        self.panelBottomRow = panelBottomRow
        super.init(frame: .zero)

        backgroundColor = UIColor.systemGray5
        for view in [stripView, qwertyView, numericView, symbolsView, panelView, panelBottomRow] {
            view.translatesAutoresizingMaskIntoConstraints = false
            addSubview(view)
        }

        NSLayoutConstraint.activate([
            stripView.topAnchor.constraint(equalTo: topAnchor),
            stripView.leadingAnchor.constraint(equalTo: leadingAnchor),
            stripView.trailingAnchor.constraint(equalTo: trailingAnchor),
            stripView.heightAnchor.constraint(equalToConstant: metrics.stripHeight),

            qwertyView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            qwertyView.leadingAnchor.constraint(equalTo: leadingAnchor),
            qwertyView.trailingAnchor.constraint(equalTo: trailingAnchor),
            qwertyView.bottomAnchor.constraint(equalTo: bottomAnchor),

            numericView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            numericView.leadingAnchor.constraint(equalTo: leadingAnchor),
            numericView.trailingAnchor.constraint(equalTo: trailingAnchor),
            numericView.bottomAnchor.constraint(equalTo: bottomAnchor),

            symbolsView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            symbolsView.leadingAnchor.constraint(equalTo: leadingAnchor),
            symbolsView.trailingAnchor.constraint(equalTo: trailingAnchor),
            symbolsView.bottomAnchor.constraint(equalTo: bottomAnchor),

            panelView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            panelView.leadingAnchor.constraint(equalTo: leadingAnchor),
            panelView.trailingAnchor.constraint(equalTo: trailingAnchor),

            panelBottomRow.topAnchor.constraint(equalTo: panelBottomAnchorGuide.topAnchor, constant: metrics.panelBottomRowTopSpacing),
            panelBottomRow.leadingAnchor.constraint(equalTo: leadingAnchor, constant: metrics.keyHorizontalInset),
            panelBottomRow.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -metrics.keyHorizontalInset),
            panelBottomRow.heightAnchor.constraint(equalToConstant: metrics.panelBottomRowHeight),
        ])

        apply(.qwerty)
    }

    required init?(coder: NSCoder) { fatalError("use init(metrics:stripView:qwertyView:numericView:symbolsView:panelView:panelBottomRow:panelBottomAnchorGuide:)") }

    func apply(_ state: KeyboardState) {
        let visibility = KeyboardLayerVisibility(state: state)
        qwertyView.isHidden = !visibility.showsQwerty
        numericView.isHidden = !visibility.showsNumeric
        symbolsView.isHidden = !visibility.showsSymbols
        panelView.isHidden = !visibility.showsPanel
        panelBottomRow.isHidden = !visibility.showsPanel
    }

    func render(_ state: IosRenderState, romanHint: String, keyboardState: KeyboardState) {
        stripDisplay.render(state, romanBuffer: romanHint)
        switch keyboardState {
        case .panel:
            panelDisplay.render(state)
        case .charPick:
            panelDisplay.renderCharPickCandidates(state.candidates)
        default:
            break
        }
    }

    func clearStrip() {
        stripDisplay.clear()
    }

    func renderCharPickAlphabet() {
        panelDisplay.renderCharPickAlphabet()
    }
}
