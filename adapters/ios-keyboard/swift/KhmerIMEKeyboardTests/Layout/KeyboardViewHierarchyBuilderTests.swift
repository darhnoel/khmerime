import UIKit
import XCTest

final class KeyboardViewHierarchyBuilderTests: XCTestCase {
    func test_buildCreatesRootWithKeyboardViewsAndInitialQwertyState() {
        let target = ActionTarget()
        let delegate = PanelDelegate()
        let hierarchy = KeyboardViewHierarchyBuilder(
            metrics: KeyboardLayoutMetrics(device: .phone),
            isIPad: false,
            target: target,
            globeKeyTag: 99,
            enKeyTag: 98,
            actions: ActionTarget.actions
        ).build(panelDelegate: delegate)

        XCTAssertTrue(hierarchy.rootView.subviews.contains(hierarchy.stripView))
        XCTAssertTrue(hierarchy.rootView.subviews.contains(hierarchy.qwertyView))
        XCTAssertTrue(hierarchy.rootView.subviews.contains(hierarchy.numericView))
        XCTAssertTrue(hierarchy.rootView.subviews.contains(hierarchy.symbolsView))
        XCTAssertTrue(hierarchy.rootView.subviews.contains(hierarchy.panelView))
        XCTAssertTrue(hierarchy.rootView.subviews.contains(hierarchy.panelBottomRow))
        XCTAssertTrue(hierarchy.panelView.delegate === delegate)

        XCTAssertFalse(hierarchy.qwertyView.isHidden)
        XCTAssertTrue(hierarchy.numericView.isHidden)
        XCTAssertTrue(hierarchy.symbolsView.isHidden)
        XCTAssertTrue(hierarchy.panelView.isHidden)
        XCTAssertTrue(hierarchy.panelBottomRow.isHidden)

        hierarchy.rootView.apply(.panel)

        XCTAssertTrue(hierarchy.qwertyView.isHidden)
        XCTAssertFalse(hierarchy.panelView.isHidden)
        XCTAssertFalse(hierarchy.panelBottomRow.isHidden)
    }
}

private final class ActionTarget: NSObject {
    static let actions = KeyboardLayerActions(
        nextKeyboard: #selector(nextKeyboardTapped),
        letter: #selector(letterTapped(_:)),
        symbol: #selector(symbolKeyTapped(_:)),
        period: #selector(periodTapped),
        backspace: #selector(backspaceTapped),
        space: #selector(spaceTapped),
        returnKey: #selector(returnTapped),
        togglePanel: #selector(togglePanelTapped),
        toggleEnglish: #selector(toggleEnglishTapped),
        numeric: #selector(numericTapped),
        symbols: #selector(symbolsTapped),
        abc: #selector(abcTapped)
    )

    @objc func nextKeyboardTapped() {}
    @objc func letterTapped(_ sender: UIButton) {}
    @objc func symbolKeyTapped(_ sender: UIButton) {}
    @objc func periodTapped() {}
    @objc func backspaceTapped() {}
    @objc func spaceTapped() {}
    @objc func returnTapped() {}
    @objc func togglePanelTapped() {}
    @objc func toggleEnglishTapped() {}
    @objc func numericTapped() {}
    @objc func symbolsTapped() {}
    @objc func abcTapped() {}
}

private final class PanelDelegate: CandidatePanelDelegate {
    func candidatePanel(_ panel: CandidatePanelView, didTapChipAt index: Int) {}
    func candidatePanel(_ panel: CandidatePanelView, didRequestEditAt index: Int) {}
    func candidatePanel(_ panel: CandidatePanelView, didSelectCandidateAt index: Int) {}
    func candidatePanelDidDismiss(_ panel: CandidatePanelView) {}
    func candidatePanelDidEnterCharPick(_ panel: CandidatePanelView) {}
    func candidatePanel(_ panel: CandidatePanelView, didTapCharPickLetter letter: Character) {}
}
