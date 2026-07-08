import UIKit
import XCTest

final class KeyboardViewHierarchyBuilderTests: XCTestCase {
    func test_buildCreatesRootWithKeyboardViewsAndInitialQwertyState() {
        let target = ActionTarget()
        let hierarchy = KeyboardViewHierarchyBuilder(
            metrics: KeyboardLayoutMetrics(device: .phone),
            isIPad: false,
            target: target,
            globeKeyTag: 99,
            enKeyTag: 98,
            actions: ActionTarget.actions
        ).build()

        XCTAssertTrue(hierarchy.rootView.subviews.contains(hierarchy.stripView))
        XCTAssertTrue(hierarchy.rootView.subviews.contains(hierarchy.qwertyView))
        XCTAssertTrue(hierarchy.rootView.subviews.contains(hierarchy.numericView))
        XCTAssertTrue(hierarchy.rootView.subviews.contains(hierarchy.symbolsView))

        XCTAssertFalse(hierarchy.qwertyView.isHidden)
        XCTAssertTrue(hierarchy.numericView.isHidden)
        XCTAssertTrue(hierarchy.symbolsView.isHidden)

        hierarchy.rootView.apply(.charPick)

        XCTAssertFalse(hierarchy.qwertyView.isHidden)
    }

    func test_buildPlacesCandidateSurfaceInTheCandidateRowSlot() {
        // ADR-0014: the slot hosts the Phrase Wheel (composition) + the word candidate
        // row (CharPick), inside a CandidateSurfaceView.
        let target = ActionTarget()

        let hierarchy = KeyboardViewHierarchyBuilder(
            metrics: KeyboardLayoutMetrics(device: .phone),
            isIPad: false,
            target: target,
            globeKeyTag: 99,
            enKeyTag: 98,
            actions: ActionTarget.actions
        ).build()

        XCTAssertTrue(hierarchy.candidateRowView is CandidateSurfaceView,
            "the candidate-surface host should occupy the candidate-row slot")
    }

    func test_buildWiresWheelCommitThroughTheCallback() {
        let target = ActionTarget()
        var committed: Int?

        let hierarchy = KeyboardViewHierarchyBuilder(
            metrics: KeyboardLayoutMetrics(device: .phone),
            isIPad: false,
            target: target,
            globeKeyTag: 99,
            enKeyTag: 98,
            actions: ActionTarget.actions
        ).build(
            phraseCommit: { committed = $0 }
        )

        (hierarchy.candidateRowView as? CandidateSurfaceView)?.onPhraseCommitted?(3)

        XCTAssertEqual(committed, 3,
            "tapping a wheel card must route through the build callback → commitPhrase")
    }

    func test_buildWiresCharPickSelectionThroughTheCallback() {
        let target = ActionTarget()
        var selected: Int?

        let hierarchy = KeyboardViewHierarchyBuilder(
            metrics: KeyboardLayoutMetrics(device: .phone),
            isIPad: false,
            target: target,
            globeKeyTag: 99,
            enKeyTag: 98,
            actions: ActionTarget.actions
        ).build(
            candidateSelection: { selected = $0 }
        )

        (hierarchy.candidateRowView as? CandidateSurfaceView)?.onCandidateSelected?(2)

        XCTAssertEqual(selected, 2,
            "tapping a CharPick character candidate must route through the build callback")
    }
}

private final class ActionTarget: NSObject {
    static let actions = KeyboardLayerActions(
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
