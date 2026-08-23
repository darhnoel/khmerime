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
        // Globe uses an SF Symbol image, so it has no title (empty slot). Row is
        // [globe][✦][space][EN][⏎] (ADR-0022 rebalance); "." dropped.
        XCTAssertEqual(buttonTitles(in: rows[3]), ["", "✦", "space", "EN", "⏎"])
    }

    func test_numericAndSymbolsLayersBuildExpectedModeSwitchRows() {
        let numericRows = standardRows(in: factory.buildNumericView())
        let symbolsRows = standardRows(in: factory.buildSymbolsView())

        XCTAssertEqual(buttonTitles(in: numericRows[2]), ["ABC", ".", ",", "?", "!", "'", "⌫"])
        XCTAssertEqual(buttonTitles(in: numericRows[3]), ["", "#+=", "space", "EN", "⏎"])
        XCTAssertEqual(buttonTitles(in: symbolsRows[2]), ["123", ".", ",", "?", "!", "'", "⌫"])
        XCTAssertEqual(buttonTitles(in: symbolsRows[3]), ["", "ABC", "space", "EN", "⏎"])
    }

    func test_visiblePunctuationAndSymbolsRouteThroughLiteralAction() {
        let numericRows = standardRows(in: factory.buildNumericView())
        let symbolButtons = numericRows[0].subviews.compactMap { $0 as? UIButton }
        let literalButtons: [GlassKeyButton] = symbolButtons.compactMap { $0 as? GlassKeyButton }

        literalButtons.forEach { button in
            button.configureForTesting(runner: { _, to, _, onUpdate in onUpdate(to) })
            button.touchesBegan(Set(), with: nil)
        }

        // "." is no longer on the Qwerty row (ADR-0022); the digits still route
        // through the rapid literal touch-down path.
        XCTAssertEqual(target.literalInputs, ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"])
    }

    func test_numericLayerSwappedModeButtonsKeepTheirActions() {
        let rows = standardRows(in: factory.buildNumericView())
        let row3Buttons = (rows[2] as? UIStackView)?.arrangedSubviews.compactMap { $0 as? UIButton } ?? []
        let bottomButtons = (rows[3] as? UIStackView)?.arrangedSubviews.compactMap { $0 as? UIButton } ?? []

        XCTAssertEqual(row3Buttons.first?.title(for: .normal), "ABC")
        XCTAssertEqual(row3Buttons.first.map(actionNames(on:)), ["abcTapped"])
        // Bottom row is [globe][#+=][space][EN][⏎]; #+= sits at index 1 now.
        XCTAssertEqual(bottomButtons[1].title(for: .normal), "#+=")
        XCTAssertEqual(actionNames(on: bottomButtons[1]), ["symbolsTapped"])
    }

    func test_bottomRowTagsGlobeAndEnAndWiresActionsToTarget() {
        let row = factory.makeBottomRow(leftLabel: "123", leftAction: ActionTarget.actions.numeric, includePeriod: true)
        let buttons = row.arrangedSubviews.compactMap { $0 as? UIButton }

        // Row order: [globe][123][space][EN][⏎] (ADR-0022).
        XCTAssertEqual(buttons[0].tag, 42, "globe must carry globeKeyTag")
        XCTAssertEqual(buttons[3].tag, 43, "EN must carry enKeyTag")
        // Globe has no factory-wired tap action: KeyboardViewController wires
        // handleInputModeList(from:with:) for .allTouchEvents at runtime.
        XCTAssertEqual(actionNames(on: buttons[0]), [])
        XCTAssertEqual(actionNames(on: buttons[1]), ["numericTapped"])
        XCTAssertEqual(actionNames(on: buttons[2]), ["spaceTapped"])
        XCTAssertEqual(actionNames(on: buttons[3]), ["toggleEnglishTapped"])
        XCTAssertEqual(actionNames(on: buttons[4]), ["returnTapped"])
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

        // Bottom row is [globe][123][space][EN][⏎] (ADR-0022); no character keys.
        XCTAssertEqual(letter?.previewLabel, "K")
        XCTAssertEqual(symbol?.previewLabel, "?")
        XCTAssertNil(bottomKeys[0].previewLabel, "globe must not show a key preview popup")
        XCTAssertNil(bottomKeys[1].previewLabel, "123 is a mode key, not a character key")
        XCTAssertNil(bottomKeys[2].previewLabel, "space is a control key")
        XCTAssertNil(bottomKeys[3].previewLabel, "EN is a mode key, not a character key")
        XCTAssertNil(bottomKeys[4].previewLabel, "return is a control key")
    }

    func test_qwertyCharacterGridKeepsLettersUniformAndWidensEdgeControls() {
        let layout = QwertyCharacterGridLayout(availableWidth: 390, spacing: 6)

        XCTAssertGreaterThan(layout.row2LeadingSideInset, 0)
        XCTAssertEqual(layout.row2LeadingSideInset, layout.row2TrailingSideInset, accuracy: 0.001)
        XCTAssertEqual(layout.row3LeadingControlWidth, layout.row3TrailingControlWidth, accuracy: 0.001)
        XCTAssertGreaterThan(layout.row3LeadingControlWidth, layout.characterKeyWidth)

        XCTAssertEqual(layout.row1ConsumedWidth, 390, accuracy: 0.001)
        XCTAssertEqual(layout.row2ConsumedWidth, 390, accuracy: 0.001)
        XCTAssertEqual(layout.row3ConsumedWidth, 390, accuracy: 0.001)
    }

    func test_qwertyLayerAppliesCharacterGridAtLayoutTime() {
        let layer = factory.buildQwertyView()
        layer.frame = CGRect(x: 0, y: 0, width: 390, height: 216)
        layer.layoutIfNeeded()

        let buttons = buttonsByTitle(in: layer)

        XCTAssertEqual(buttons["A"]?.bounds.width ?? 0, buttons["Q"]?.bounds.width ?? 0, accuracy: 0.001)
        XCTAssertEqual(buttons["Z"]?.bounds.width ?? 0, buttons["Q"]?.bounds.width ?? 0, accuracy: 0.001)
        XCTAssertEqual(buttons["123"]?.bounds.width ?? 0, buttons["⌫"]?.bounds.width ?? 0, accuracy: 0.5)
        XCTAssertGreaterThan(buttons["⌫"]?.bounds.width ?? 0, buttons["Q"]?.bounds.width ?? 0)
        XCTAssertGreaterThan(buttons["A"]?.frame.minX ?? 0, buttons["Q"]?.frame.minX ?? 0)
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

    private func buttonsByTitle(in view: UIView) -> [String: UIButton] {
        var result: [String: UIButton] = [:]
        if let button = view as? UIButton, let title = button.title(for: .normal), !title.isEmpty {
            result[title] = button
        }
        for subview in view.subviews {
            result.merge(buttonsByTitle(in: subview)) { current, _ in current }
        }
        return result
    }

    private func actionNames(on button: UIButton) -> [String] {
        button.actions(forTarget: target, forControlEvent: .touchUpInside) ?? []
    }
}

private final class ActionTarget: NSObject {
    var literalInputs: [String] = []

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
    @objc func literalKeyTapped(_ sender: UIButton) {
        literalInputs.append(sender.title(for: .normal) ?? "")
    }
    @objc func backspaceTapped() {}
    @objc func spaceTapped() {}
    @objc func returnTapped() {}
    @objc func togglePanelTapped() {}
    @objc func toggleEnglishTapped() {}
    @objc func numericTapped() {}
    @objc func symbolsTapped() {}
    @objc func abcTapped() {}
}
