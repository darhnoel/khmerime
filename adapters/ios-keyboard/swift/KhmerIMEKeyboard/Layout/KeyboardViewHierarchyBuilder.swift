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
        candidateRowSelection: ((Int) -> Void)? = nil
    ) -> KeyboardViewHierarchy {
        let stripView = StripView()
        stripView.translatesAutoresizingMaskIntoConstraints = false

        // ADR-0014: the Phrase Wheel is the default candidate surface, occupying the
        // candidate-row slot. `candidateRowSelection` (word-level tap) is unused here;
        // wheel selection is driven by scroll snapping (wired in a later slice).
        _ = candidateRowSelection
        let candidateRowView = PhraseWheelView()
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
