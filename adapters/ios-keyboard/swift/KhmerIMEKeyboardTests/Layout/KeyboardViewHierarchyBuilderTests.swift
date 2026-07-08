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

    func test_buildPlacesPhraseWheelInTheCandidateRowSlot() {
        // ADR-0014: the Phrase Wheel is the default candidate surface; the word-level
        // candidate row moves to Level-2 editing (a later slice).
        let target = ActionTarget()

        let hierarchy = KeyboardViewHierarchyBuilder(
            metrics: KeyboardLayoutMetrics(device: .phone),
            isIPad: false,
            target: target,
            globeKeyTag: 99,
            enKeyTag: 98,
            actions: ActionTarget.actions
        ).build()

        XCTAssertTrue(hierarchy.candidateRowView is PhraseWheelView,
            "the Phrase Wheel should occupy the default candidate-row slot")
    }

    func test_buildWiresWheelSelectionThroughTheCallback() {
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
            candidateRowSelection: { selected = $0 }
        )

        (hierarchy.candidateRowView as? PhraseWheelView)?.onPhraseSelected?(3)

        XCTAssertEqual(selected, 3,
            "scrolling the wheel to a card must route through the build callback → selectPhrase")
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
