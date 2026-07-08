import UIKit

struct KeyboardViewHierarchy {
    let stripView: StripView
    let candidateRowView: UIView & KeyboardCandidateRowDisplaying
    let qwertyView: UIView
    let numericView: UIView
    let symbolsView: UIView
    let rootView: KeyboardRootView
}

struct KeyboardViewHierarchyBuilder {
    let metrics: KeyboardLayoutMetrics
    let isIPad: Bool
    let target: AnyObject
    let globeKeyTag: Int
    let enKeyTag: Int
    let actions: KeyboardLayerActions

    func build(
        candidateSelection: ((Int) -> Void)? = nil,
        phraseSelection: ((Int) -> Void)? = nil
    ) -> KeyboardViewHierarchy {
        let stripView = StripView()
        stripView.translatesAutoresizingMaskIntoConstraints = false

        // ADR-0014: the candidate-row slot hosts the Phrase Wheel (composition) and the
        // word candidate row (CharPick). Wheel scroll → phraseSelection; CharPick tap →
        // candidateSelection.
        let candidateRowView = CandidateSurfaceView()
        candidateRowView.onPhraseSelected = phraseSelection
        candidateRowView.onCandidateSelected = candidateSelection
        candidateRowView.translatesAutoresizingMaskIntoConstraints = false

        let layerFactory = KeyboardLayerFactory(
            metrics: metrics,
            isIPad: isIPad,
            target: target,
            globeKeyTag: globeKeyTag,
            enKeyTag: enKeyTag,
            actions: actions
        )
        let qwertyView = layerFactory.buildQwertyView()
        let numericView = layerFactory.buildNumericView()
        let symbolsView = layerFactory.buildSymbolsView()

        let rootView = KeyboardRootView(
            metrics: metrics,
            stripView: stripView,
            qwertyView: qwertyView,
            numericView: numericView,
            symbolsView: symbolsView,
            candidateRowView: candidateRowView
        )
        rootView.translatesAutoresizingMaskIntoConstraints = false

        return KeyboardViewHierarchy(
            stripView: stripView,
            candidateRowView: candidateRowView,
            qwertyView: qwertyView,
            numericView: numericView,
            symbolsView: symbolsView,
            rootView: rootView
        )
    }
}
