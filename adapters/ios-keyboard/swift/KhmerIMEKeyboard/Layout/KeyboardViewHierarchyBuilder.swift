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

        // The candidate-row slot hosts the Phrase Wheel (composition) and the word
        // candidate row (CharPick). Wheel tap → phraseSelection; CharPick tap → candidateSelection.
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
        wireKeyPreviewCallbacks(rootView: rootView, keyLayers: [qwertyView, numericView, symbolsView])

        return KeyboardViewHierarchy(
            stripView: stripView,
            candidateRowView: candidateRowView,
            qwertyView: qwertyView,
            numericView: numericView,
            symbolsView: symbolsView,
            rootView: rootView
        )
    }

    private func wireKeyPreviewCallbacks(rootView: KeyboardRootView, keyLayers: [UIView]) {
        for key in keyLayers.flatMap({ glassKeys(in: $0) }) {
            key.onPreviewChanged = { [weak rootView] sourceKey, label in
                guard let label else {
                    rootView?.hideKeyPreview()
                    return
                }
                rootView?.showKeyPreview(label: label, from: sourceKey)
            }
        }
    }

    private func glassKeys(in view: UIView) -> [GlassKeyButton] {
        var result = view.subviews.compactMap { $0 as? GlassKeyButton }
        for subview in view.subviews {
            result += glassKeys(in: subview)
        }
        return result
    }
}
