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

    // Height constraints for the two chrome rows. Driven by KeyboardChrome.Rows
    // so roman composition can reserve both rows while CharPick reserves only
    // the candidate row.
    let stripHeightConstraint: NSLayoutConstraint
    let candidateRowHeightConstraint: NSLayoutConstraint
    private let stripReservedHeight: CGFloat
    private let candidateRowReservedHeight: CGFloat

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
        self.stripReservedHeight = metrics.stripHeight
        self.candidateRowReservedHeight = metrics.candidateRowHeight
        self.stripHeightConstraint = stripView.heightAnchor.constraint(equalToConstant: metrics.stripHeight)
        self.candidateRowHeightConstraint = candidateRowView.heightAnchor.constraint(equalToConstant: metrics.candidateRowHeight)
        // The key area never changes size — it equals the keyboard height with the
        // chrome collapsed. Keys are bottom-pinned to this fixed height.
        let keyAreaHeight = metrics.idleKeyboardHeight
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
            stripHeightConstraint,

            candidateRowView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            candidateRowView.leadingAnchor.constraint(equalTo: leadingAnchor),
            candidateRowView.trailingAnchor.constraint(equalTo: trailingAnchor),
            candidateRowHeightConstraint,

            // Key layers are bottom-anchored with a FIXED height (the key area is
            // always idleKeyboardHeight, in both idle and composing). They are NOT
            // pinned below the chrome, so collapsing/expanding the strip + candidate
            // row above them never moves or resizes the keys — only the chrome
            // animates. Required priority keeps the keys rigid; the 999 host height
            // absorbs any sub-pixel rounding during the animation.
            qwertyView.heightAnchor.constraint(equalToConstant: keyAreaHeight),
            qwertyView.leadingAnchor.constraint(equalTo: leadingAnchor),
            qwertyView.trailingAnchor.constraint(equalTo: trailingAnchor),
            qwertyView.bottomAnchor.constraint(equalTo: bottomAnchor),

            numericView.heightAnchor.constraint(equalToConstant: keyAreaHeight),
            numericView.leadingAnchor.constraint(equalTo: leadingAnchor),
            numericView.trailingAnchor.constraint(equalTo: trailingAnchor),
            numericView.bottomAnchor.constraint(equalTo: bottomAnchor),

            symbolsView.heightAnchor.constraint(equalToConstant: keyAreaHeight),
            symbolsView.leadingAnchor.constraint(equalTo: leadingAnchor),
            symbolsView.trailingAnchor.constraint(equalTo: trailingAnchor),
            symbolsView.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])

        apply(.qwerty)
        setChromeRows(.none)   // appear collapsed; expands on first keystroke
    }

    required init?(coder: NSCoder) { fatalError("use init(metrics:stripView:qwertyView:numericView:symbolsView:candidateRowView:)") }

    // Applies exact chrome row heights. The caller animates layout and adjusts
    // the host height alongside these constraints.
    func setChromeRows(_ rows: KeyboardChrome.Rows) {
        switch rows {
        case .none:
            stripHeightConstraint.constant = 0
            candidateRowHeightConstraint.constant = 0
        case .stripOnly:
            stripHeightConstraint.constant = stripReservedHeight
            candidateRowHeightConstraint.constant = 0
        case .candidateOnly:
            stripHeightConstraint.constant = 0
            candidateRowHeightConstraint.constant = candidateRowReservedHeight
        case .stripAndCandidate:
            stripHeightConstraint.constant = stripReservedHeight
            candidateRowHeightConstraint.constant = candidateRowReservedHeight
        }
    }

    func apply(_ state: KeyboardState) {
        let visibility = KeyboardLayerVisibility(state: state)
        qwertyView.isHidden = !visibility.showsQwerty
        numericView.isHidden = !visibility.showsNumeric
        symbolsView.isHidden = !visibility.showsSymbols
        candidateRowView.isHidden = !visibility.showsCandidateRow
    }

    func render(_ state: IosRenderState, romanHint: String, keyboardState: KeyboardState) {
        stripDisplay.render(state, romanBuffer: romanHint)
        let presentation: CandidateRowPresentation = keyboardState == .charPick ? .charPick : .composition
        candidateRowDisplay.render(state, presentation: presentation)
    }

    func clearStrip() {
        stripDisplay.clear()
        candidateRowDisplay.clear()
    }
}
