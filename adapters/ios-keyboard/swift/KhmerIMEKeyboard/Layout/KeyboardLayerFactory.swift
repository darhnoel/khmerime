import UIKit

// KeyboardLayout
// ==============
// Builds the three standard keyboard layers (QWERTY, 123, #+=) and the shared
// bottom row. All views are purely declarative: they know nothing about the
// session; user interactions are forwarded to an injected target through
// selectors owned by KeyboardViewController.swift.
//
// Android equivalent
// ------------------
// These builders correspond to inflating separate layout XMLs (or building
// views programmatically) for each keyboard layer. In Android you would
// typically inflate three layouts and toggle their visibility:
//
//   private fun buildQwertyView(): View = layoutInflater.inflate(R.layout.keyboard_qwerty, root, false)
//   private fun buildNumericView(): View = layoutInflater.inflate(R.layout.keyboard_numeric, root, false)
//   private fun buildSymbolsView(): View = layoutInflater.inflate(R.layout.keyboard_symbols, root, false)
//
// Key row anatomy:
//   QWERTY row 1: q w e r t y u i o p       (10 letter keys, fillEqually)
//   QWERTY row 2:   a s d f g h j k l       (9 letter keys, fillEqually, inset)
//   QWERTY row 3: 123 z x c v b n m ⌫        (special | letters | special)
//   Bottom row:   ✦ | space (flex) | . | ⏎
//
//   123 row 3:    #+= | . , ? ! ' | ⌫
//   #+= row 3:    123 | . , ? ! ' | ⌫
//   ABC bottom:   ABC | space (flex) | ⏎      (no period: it's in row 3)

struct KeyboardLayerActions {
    let letter: Selector
    let symbol: Selector
    let period: Selector
    let backspace: Selector
    let space: Selector
    let returnKey: Selector
    let togglePanel: Selector
    let toggleEnglish: Selector
    let numeric: Selector
    let symbols: Selector
    let abc: Selector
}

struct KeyboardLayerFactory {
    let metrics: KeyboardLayoutMetrics
    let isIPad: Bool
    let target: AnyObject
    let globeKeyTag: Int
    let enKeyTag: Int
    let actions: KeyboardLayerActions

    // MARK: - Adaptive Dimensions

    private var specialKeyW: CGFloat { metrics.specialKeyWidth }
    private var returnKeyW: CGFloat { metrics.returnKeyWidth }
    private var wideSpecialKeyW: CGFloat { metrics.wideSpecialKeyWidth }

    // MARK: - Standard Layers

    func buildQwertyView() -> UIView {
        buildStandardView(rows: [
            makeLetterRow(["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"]),
            makeQwertyRow2(),
            makeQwertyRow3(),
            makeBottomRow(leftLabel: "✦", leftAction: actions.togglePanel, includePeriod: true),
        ])
    }

    func buildNumericView() -> UIView {
        buildStandardView(rows: [
            makeSymbolRow(["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]),
            makeSymbolRow(["-", "/", ":", ";", "(", ")", "¥", "&", "@", "\""]),
            makeSpecialSideRow(leftLabel: "ABC", leftAction: actions.abc, leftWidth: wideSpecialKeyW),
            makeBottomRow(leftLabel: "#+=", leftAction: actions.symbols, includePeriod: false),
        ])
    }

    func buildSymbolsView() -> UIView {
        buildStandardView(rows: [
            makeSymbolRow(["[", "]", "{", "}", "#", "%", "^", "*", "+", "="]),
            makeSymbolRow(["_", "\\", "|", "~", "<", ">", "€", "£", "¥", "•"]),
            makeSpecialSideRow(leftLabel: "123", leftAction: actions.numeric, leftWidth: wideSpecialKeyW),
            makeBottomRow(leftLabel: "ABC", leftAction: actions.abc, includePeriod: false),
        ])
    }

    // MARK: - Shared Bottom Row

    // Used by QWERTY, the panel, and (without period) by 123 and #+=.
    // The globe key is always added but hidden when iOS doesn't require us to
    // show our own switcher (viewWillLayoutSubviews manages show/hide via tag).
    func makeBottomRow(leftLabel: String, leftAction: Selector, includePeriod: Bool) -> UIStackView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fill

        let globeBtn = makeGlobeKey()
        globeBtn.widthAnchor.constraint(equalToConstant: specialKeyW).isActive = true
        row.addArrangedSubview(globeBtn)

        let enBtn = makeSpecialKey("EN", action: actions.toggleEnglish)
        enBtn.titleLabel?.font = .systemFont(ofSize: 13, weight: .medium)
        enBtn.widthAnchor.constraint(equalToConstant: specialKeyW).isActive = true
        enBtn.tag = enKeyTag
        row.addArrangedSubview(enBtn)

        let leftBtn = makeSpecialKey(leftLabel, action: leftAction)
        leftBtn.titleLabel?.font = .systemFont(ofSize: 13, weight: .medium)
        leftBtn.widthAnchor.constraint(equalToConstant: specialKeyW).isActive = true

        // Space bar stretches to fill the remaining width.
        let spaceBtn = makeSpecialKey("space", action: actions.space)
        spaceBtn.titleLabel?.font = .systemFont(ofSize: 13)
        spaceBtn.setContentHuggingPriority(.init(rawValue: 1), for: .horizontal)
        spaceBtn.setContentCompressionResistancePriority(.init(rawValue: 1), for: .horizontal)

        let returnBtn = makeSpecialKey("⏎", action: actions.returnKey)
        returnBtn.widthAnchor.constraint(equalToConstant: returnKeyW).isActive = true

        row.addArrangedSubview(leftBtn)
        row.addArrangedSubview(spaceBtn)
        if includePeriod {
            let periodBtn = makeSpecialKey(".", action: actions.period, previewLabel: ".")
            periodBtn.widthAnchor.constraint(equalToConstant: specialKeyW).isActive = true
            row.addArrangedSubview(periodBtn)
        }
        row.addArrangedSubview(returnBtn)
        return row
    }

    // MARK: - Row Builders

    private func makeQwertyRow2() -> UIStackView {
        QwertyCharacterGridRowView.centeredLetterRow(
            ["a", "s", "d", "f", "g", "h", "j", "k", "l"].map { makeLetterKey($0) },
            spacing: 6
        )
    }

    private func makeQwertyRow3() -> UIView {
        let toggleBtn = makeSpecialKey("123", action: actions.numeric)
        let letters = ["z", "x", "c", "v", "b", "n", "m"].map { makeLetterKey($0) }
        let backBtn = makeBackspaceButton()

        return QwertyCharacterGridRowView.edgeControlRow(
            leadingControl: toggleBtn,
            letters: letters,
            trailingControl: backBtn,
            spacing: 6
        )
    }

    // Row 3 of 123 and #+=: [leftSpecial | . , ? ! ' | ⌫]
    private func makeSpecialSideRow(leftLabel: String, leftAction: Selector, leftWidth: CGFloat) -> UIView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fill

        let leftBtn = makeSpecialKey(leftLabel, action: leftAction)
        leftBtn.titleLabel?.font = .systemFont(ofSize: 13, weight: .medium)
        leftBtn.widthAnchor.constraint(equalToConstant: leftWidth).isActive = true

        let mid = makeSymbolRow([".", ",", "?", "!", "'"])
        mid.setContentHuggingPriority(.init(rawValue: 1), for: .horizontal)

        let backBtn = makeBackspaceButton()
        backBtn.widthAnchor.constraint(equalToConstant: specialKeyW).isActive = true

        row.addArrangedSubview(leftBtn)
        row.addArrangedSubview(mid)
        row.addArrangedSubview(backBtn)
        return row
    }

    func makeBackspaceButton() -> BackspaceButton {
        let btn = BackspaceButton()
        btn.setTitle("⌫", for: .normal)
        KeyStyle.applySpecial(btn, isIPad: isIPad)
        return btn
    }

    // MARK: - Key Factories

    func makeLetterRow(_ keys: [String]) -> UIStackView {
        let row = UIStackView(arrangedSubviews: keys.map { makeLetterKey($0) })
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fillEqually
        return row
    }

    func makeSymbolRow(_ keys: [String]) -> UIStackView {
        let row = UIStackView(arrangedSubviews: keys.map { makeSymbolKey($0) })
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fillEqually
        return row
    }

    func makeLetterKey(_ letter: String) -> UIButton {
        let btn = GlassKeyButton(frame: .zero)
        btn.setTitle(letter.uppercased(), for: .normal)
        btn.previewLabel = letter.uppercased()
        KeyStyle.applyLetter(btn, isIPad: isIPad)
        btn.onPress = { [weak target, weak btn] in
            _ = target?.perform(actions.letter, with: btn)
        }
        return btn
    }

    func makeSymbolKey(_ symbol: String) -> UIButton {
        let btn = GlassKeyButton(frame: .zero)
        btn.setTitle(symbol, for: .normal)
        btn.previewLabel = symbol
        KeyStyle.applySymbol(btn, isIPad: isIPad)
        btn.onPress = { [weak target, weak btn] in
            _ = target?.perform(actions.symbol, with: btn)
        }
        return btn
    }

    func makeSpecialKey(_ title: String, action: Selector, previewLabel: String? = nil) -> UIButton {
        let btn = GlassKeyButton(frame: .zero)
        btn.setTitle(title, for: .normal)
        btn.previewLabel = previewLabel
        KeyStyle.applySpecial(btn, isIPad: isIPad)
        btn.addTarget(target, action: action, for: .touchUpInside)
        return btn
    }

    // The next-keyboard key. Uses the SF Symbol "globe" sized and tinted to match
    // the other special keys. GlobeKeyButton handles long-press detection internally
    // via timer; KeyboardViewController wires onShortTap / onLongPress callbacks.
    func makeGlobeKey() -> UIButton {
        let btn = GlobeKeyButton(frame: .zero)
        let config = UIImage.SymbolConfiguration(pointSize: isIPad ? 17 : 15, weight: .medium)
        btn.setImage(UIImage(systemName: "globe", withConfiguration: config), for: .normal)
        KeyStyle.applySpecial(btn, isIPad: isIPad)
        btn.tintColor = .label
        btn.accessibilityLabel = "Next Keyboard"
        btn.tag = globeKeyTag
        return btn
    }

    // MARK: - Private Helpers

    // Wraps an array of row views in a vertical UIStackView with standard
    // keyboard insets. All three standard layers share this container.
    private func buildStandardView(rows: [UIView]) -> UIView {
        let container = UIView()
        let stack = UIStackView(arrangedSubviews: rows)
        stack.axis = .vertical
        stack.spacing = metrics.rowSpacing
        stack.distribution = .fillEqually
        stack.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: container.topAnchor, constant: metrics.keyTopInset),
            stack.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: metrics.keyHorizontalInset),
            stack.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -metrics.keyHorizontalInset),
            stack.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -metrics.keyBottomInset),
        ])
        return container
    }
}
