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
            enKeyTag: 43,
            actions: ActionTarget.actions
        )
    }

    func test_qwertyLayerBuildsExpectedRowsAndBottomKeys() {
        let layer = factory.buildQwertyView()
        let rows = standardRows(in: layer)

        XCTAssertEqual(rows.count, 4)
        XCTAssertEqual(buttonTitles(in: rows[0]), ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"])
        XCTAssertEqual(buttonTitles(in: rows[1]), ["A", "S", "D", "F", "G", "H", "J", "K", "L"])
        XCTAssertEqual(buttonTitles(in: rows[2]), ["123", "Z", "X", "C", "V", "B", "N", "M", "⌫"])
        // Globe uses an SF Symbol image, so it has no title (empty slot).
        XCTAssertEqual(buttonTitles(in: rows[3]), ["", "EN", "✦", "space", ".", "⏎"])
    }

    func test_numericAndSymbolsLayersBuildExpectedModeSwitchRows() {
        let numericRows = standardRows(in: factory.buildNumericView())
        let symbolsRows = standardRows(in: factory.buildSymbolsView())

        XCTAssertEqual(buttonTitles(in: numericRows[2]), ["#+=", ".", ",", "?", "!", "'", "⌫"])
        XCTAssertEqual(buttonTitles(in: numericRows[3]), ["", "EN", "ABC", "space", "⏎"])
        XCTAssertEqual(buttonTitles(in: symbolsRows[2]), ["123", ".", ",", "?", "!", "'", "⌫"])
        XCTAssertEqual(buttonTitles(in: symbolsRows[3]), ["", "EN", "ABC", "space", "⏎"])
    }

    func test_bottomRowTagsGlobeAndEnAndWiresActionsToTarget() {
        let row = factory.makeBottomRow(leftLabel: "123", leftAction: ActionTarget.actions.numeric, includePeriod: true)
        let buttons = row.arrangedSubviews.compactMap { $0 as? UIButton }

        XCTAssertEqual(buttons[0].tag, 42, "globe must carry globeKeyTag")
        XCTAssertEqual(buttons[1].tag, 43, "EN must carry enKeyTag")
        // Globe has no factory-wired tap action: KeyboardViewController wires
        // handleInputModeList(from:with:) for .allTouchEvents at runtime.
        XCTAssertEqual(actionNames(on: buttons[0]), [])
        XCTAssertEqual(actionNames(on: buttons[1]), ["toggleEnglishTapped"])
        XCTAssertEqual(actionNames(on: buttons[2]), ["numericTapped"])
        XCTAssertEqual(actionNames(on: buttons[3]), ["spaceTapped"])
        XCTAssertEqual(actionNames(on: buttons[4]), ["periodTapped"])
        XCTAssertEqual(actionNames(on: buttons[5]), ["returnTapped"])
    }

    func test_globeKeyUsesGlobeSymbolStyledLikeOtherKeys() {
        let globe = factory.makeGlobeKey()

        XCTAssertNotNil(globe.image(for: .normal),
            "globe must use an SF Symbol image, not a text/emoji title")
        XCTAssertNil(globe.title(for: .normal),
            "globe must not carry a text label")
        XCTAssertEqual(globe.tintColor, .label,
            "globe tint must match the other keys' label color")
        XCTAssertEqual(globe.tag, 42, "globe must carry globeKeyTag")
    }

    func test_letterAndSymbolKeysUseNativeButtonsWithNativeFonts() {
        let letter = factory.makeLetterKey("k")
        let symbol = factory.makeSymbolKey("១")

        XCTAssertEqual(letter.buttonType, .custom)
        XCTAssertEqual(symbol.buttonType, .custom)
        XCTAssertEqual(letter.title(for: .normal), "K")
        XCTAssertEqual(symbol.title(for: .normal), "១")
        XCTAssertEqual(letter.titleLabel?.font.pointSize, 17)
        XCTAssertEqual(symbol.titleLabel?.font.pointSize, 17)
    }

    func test_characterProducingKeysGetPreviewLabelsButControlsDoNot() {
        let letter = factory.makeLetterKey("k") as? GlassKeyButton
        let symbol = factory.makeSymbolKey("?") as? GlassKeyButton
        let bottomRow = factory.makeBottomRow(leftLabel: "123", leftAction: ActionTarget.actions.numeric, includePeriod: true)
        let bottomKeys = bottomRow.arrangedSubviews.compactMap { $0 as? GlassKeyButton }

        XCTAssertEqual(letter?.previewLabel, "K")
        XCTAssertEqual(symbol?.previewLabel, "?")
        XCTAssertNil(bottomKeys[0].previewLabel, "globe must not show a key preview popup")
        XCTAssertNil(bottomKeys[1].previewLabel, "EN is a mode key, not a character key")
        XCTAssertNil(bottomKeys[2].previewLabel, "123 is a mode key, not a character key")
        XCTAssertNil(bottomKeys[3].previewLabel, "space is a control key")
        XCTAssertEqual(bottomKeys[4].previewLabel, ".", "period is a small punctuation key")
        XCTAssertNil(bottomKeys[5].previewLabel, "return is a control key")
    }

    func test_keyCornerRadiusFollowsAndroidGlassProportionAfterLayout() {
        let letter = factory.makeLetterKey("k")

        letter.frame = CGRect(x: 0, y: 0, width: 32, height: 44)
        letter.layoutIfNeeded()

        XCTAssertEqual(letter.layer.cornerRadius, 44 * 0.22, accuracy: 0.01)
    }

    private func standardRows(in layer: UIView) -> [UIView] {
        guard let stack = layer.subviews.compactMap({ $0 as? UIStackView }).first else {
            XCTFail("Expected layer to contain a vertical stack")
            return []
        }
        return stack.arrangedSubviews
    }

    private func buttonTitles(in view: UIView) -> [String] {
        if let button = view as? UIButton { return [button.title(for: .normal) ?? ""] }
        guard let stack = view as? UIStackView else { return [] }
        return stack.arrangedSubviews.flatMap(buttonTitles)
    }

    private func actionNames(on button: UIButton) -> [String] {
        button.actions(forTarget: target, forControlEvent: .touchUpInside) ?? []
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
