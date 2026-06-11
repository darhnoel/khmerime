import UIKit
import XCTest

final class KeyboardLayerFactoryTests: XCTestCase {
    private var target: ActionTarget!
    private var factory: KeyboardLayerFactory!

    override func setUp() {
        super.setUp()
        target = ActionTarget()
        factory = KeyboardLayerFactory(
            metrics: KeyboardLayoutMetrics(device: .phone),
            isIPad: false,
            target: target,
            globeKeyTag: 42,
            actions: ActionTarget.actions
        )
    }

    func test_qwertyLayerBuildsExpectedRowsAndBottomKeys() {
        let layer = factory.buildQwertyView()
        let rows = standardRows(in: layer)

        XCTAssertEqual(rows.count, 4)
        XCTAssertEqual(buttonTitles(in: rows[0]), ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"])
        XCTAssertEqual(buttonTitles(in: rows[1]), ["A", "S", "D", "F", "G", "H", "J", "K", "L"])
        XCTAssertEqual(buttonTitles(in: rows[2]), ["💡", "Z", "X", "C", "V", "B", "N", "M", "⌫"])
        XCTAssertEqual(buttonTitles(in: rows[3]), ["🌐", "123", "space", ".", "⏎"])
    }

    func test_numericAndSymbolsLayersBuildExpectedModeSwitchRows() {
        let numericRows = standardRows(in: factory.buildNumericView())
        let symbolsRows = standardRows(in: factory.buildSymbolsView())

        XCTAssertEqual(buttonTitles(in: numericRows[2]), ["#+=", ".", ",", "?", "!", "'", "⌫"])
        XCTAssertEqual(buttonTitles(in: numericRows[3]), ["🌐", "ABC", "space", "⏎"])
        XCTAssertEqual(buttonTitles(in: symbolsRows[2]), ["123", ".", ",", "?", "!", "'", "⌫"])
        XCTAssertEqual(buttonTitles(in: symbolsRows[3]), ["🌐", "ABC", "space", "⏎"])
    }

    func test_bottomRowTagsGlobeAndWiresActionsToTarget() {
        let row = factory.makeBottomRow(leftLabel: "123", leftAction: ActionTarget.actions.numeric, includePeriod: true)
        let buttons = row.arrangedSubviews.compactMap { $0 as? UIButton }

        XCTAssertEqual(buttons.first?.tag, 42)
        XCTAssertEqual(actionNames(on: buttons[0]), ["nextKeyboardTapped"])
        XCTAssertEqual(actionNames(on: buttons[1]), ["numericTapped"])
        XCTAssertEqual(actionNames(on: buttons[2]), ["spaceTapped"])
        XCTAssertEqual(actionNames(on: buttons[3]), ["periodTapped"])
        XCTAssertEqual(actionNames(on: buttons[4]), ["returnTapped"])
    }

    private func standardRows(in layer: UIView) -> [UIView] {
        guard let stack = layer.subviews.compactMap({ $0 as? UIStackView }).first else {
            XCTFail("Expected layer to contain a vertical stack")
            return []
        }
        return stack.arrangedSubviews
    }

    private func buttonTitles(in view: UIView) -> [String] {
        if let button = view as? UIButton {
            return [button.title(for: .normal) ?? ""]
        }
        guard let stack = view as? UIStackView else { return [] }
        return stack.arrangedSubviews.flatMap(buttonTitles)
    }

    private func actionNames(on button: UIButton) -> [String] {
        button.actions(forTarget: target, forControlEvent: .touchUpInside) ?? []
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
    @objc func numericTapped() {}
    @objc func symbolsTapped() {}
    @objc func abcTapped() {}
}
