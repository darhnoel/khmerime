import UIKit

struct KeyboardViewHierarchy {
    let stripView: StripView
    let panelView: CandidatePanelView
    let qwertyView: UIView
    let numericView: UIView
    let symbolsView: UIView
    let panelBottomRow: UIStackView
    let rootView: KeyboardRootView
}

struct KeyboardViewHierarchyBuilder {
    let metrics: KeyboardLayoutMetrics
    let isIPad: Bool
    let target: AnyObject
    let globeKeyTag: Int
    let actions: KeyboardLayerActions

    func build(panelDelegate: CandidatePanelDelegate?) -> KeyboardViewHierarchy {
        let stripView = StripView()
        stripView.translatesAutoresizingMaskIntoConstraints = false

        let panelView = CandidatePanelView(metrics: metrics)
        panelView.delegate = panelDelegate
        panelView.translatesAutoresizingMaskIntoConstraints = false

        let layerFactory = KeyboardLayerFactory(
            metrics: metrics,
            isIPad: isIPad,
            target: target,
            globeKeyTag: globeKeyTag,
            actions: actions
        )
        let qwertyView = layerFactory.buildQwertyView()
        let numericView = layerFactory.buildNumericView()
        let symbolsView = layerFactory.buildSymbolsView()

        let panelBottomRow = layerFactory.makeBottomRow(
            leftLabel: "123",
            leftAction: actions.numeric,
            includePeriod: true
        )
        panelBottomRow.translatesAutoresizingMaskIntoConstraints = false

        let rootView = KeyboardRootView(
            metrics: metrics,
            stripView: stripView,
            qwertyView: qwertyView,
            numericView: numericView,
            symbolsView: symbolsView,
            panelView: panelView,
            panelBottomRow: panelBottomRow,
            panelBottomAnchorGuide: panelView.bottomAnchorGuide
        )
        rootView.translatesAutoresizingMaskIntoConstraints = false

        return KeyboardViewHierarchy(
            stripView: stripView,
            panelView: panelView,
            qwertyView: qwertyView,
            numericView: numericView,
            symbolsView: symbolsView,
            panelBottomRow: panelBottomRow,
            rootView: rootView
        )
    }
}
