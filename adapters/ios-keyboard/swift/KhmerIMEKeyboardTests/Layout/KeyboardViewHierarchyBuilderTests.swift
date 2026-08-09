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
            phraseSelection: { selected = $0 }
        )

        (hierarchy.candidateRowView as? CandidateSurfaceView)?.onPhraseSelected?(3)

        XCTAssertEqual(selected, 3,
            "tapping a wheel card must route through the build callback → selectPhrase (not commit)")
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

    func test_buildWiresCharacterKeyPreviewEventsToTheRootOverlay() {
        let target = ActionTarget()
        let hierarchy = KeyboardViewHierarchyBuilder(
            metrics: KeyboardLayoutMetrics(device: .phone),
            isIPad: false,
            target: target,
            globeKeyTag: 99,
            enKeyTag: 98,
            actions: ActionTarget.actions
        ).build()
        hierarchy.rootView.frame = CGRect(x: 0, y: 0, width: 320, height: 260)
        hierarchy.rootView.layoutIfNeeded()

        let qKey = descendants(ofType: GlassKeyButton.self, in: hierarchy.qwertyView)
            .first { $0.title(for: .normal) == "Q" }!

        qKey.touchesBegan(Set(), with: nil)

        let popups = hierarchy.rootView.subviews.compactMap { $0 as? KeyPreviewPopupView }
        XCTAssertEqual(popups.count, 1)
        guard let popup = popups.first else { return }
        XCTAssertEqual(popup.previewLabel, "Q")

        qKey.touchesEnded(Set(), with: nil)

        XCTAssertTrue(hierarchy.rootView.subviews.compactMap { $0 as? KeyPreviewPopupView }.isEmpty)
    }

    private func descendants<T: UIView>(ofType _: T.Type, in view: UIView) -> [T] {
        var result = view.subviews.compactMap { $0 as? T }
        for subview in view.subviews {
            result += descendants(ofType: T.self, in: subview)
        }
        return result
    }
}

private final class ActionTarget: NSObject {
    static let actions = KeyboardLayerActions(
        letter: #selector(letterTapped(_:)),
        literal: #selector(literalKeyTapped(_:)),
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
    @objc func literalKeyTapped(_ sender: UIButton) {}
    @objc func backspaceTapped() {}
    @objc func spaceTapped() {}
    @objc func returnTapped() {}
    @objc func togglePanelTapped() {}
    @objc func toggleEnglishTapped() {}
    @objc func numericTapped() {}
    @objc func symbolsTapped() {}
    @objc func abcTapped() {}
}
