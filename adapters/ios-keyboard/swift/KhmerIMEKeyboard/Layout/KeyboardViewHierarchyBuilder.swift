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
        phraseCommit: ((Int) -> Void)? = nil
    ) -> KeyboardViewHierarchy {
        let stripView = StripView()
        stripView.translatesAutoresizingMaskIntoConstraints = false

        // The candidate-row slot hosts the Phrase Wheel (composition) and the word
        // candidate row (CharPick). Wheel tap → phraseCommit; CharPick tap → candidateSelection.
        let candidateRowView = CandidateSurfaceView()
        candidateRowView.onPhraseCommitted = phraseCommit
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
