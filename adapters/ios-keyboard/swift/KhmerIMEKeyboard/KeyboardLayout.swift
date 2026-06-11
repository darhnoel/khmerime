import UIKit

// KeyboardLayout
// ==============
// Builds the three standard keyboard layers (QWERTY, 123, #+=) and the shared
// bottom row. All views are purely declarative: they know nothing about the
// session; user interactions are forwarded to the ViewController via @objc
// selectors defined in KeyboardViewController.swift.
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
//   QWERTY row 3: 💡 z x c v b n m ⌫        (special | letters | special)
//   Bottom row:   123 | space (flex) | . | ⏎
//
//   123 row 3:    #+= | . , ? ! ' | ⌫
//   #+= row 3:    123 | . , ? ! ' | ⌫
//   ABC bottom:   ABC | space (flex) | ⏎      (no period: it's in row 3)

extension KeyboardViewController {

    // MARK: - Standard Layers

    func buildQwertyView() -> UIView {
        buildStandardView(rows: [
            makeLetterRow(["q","w","e","r","t","y","u","i","o","p"]),
            makeLetterRow(["a","s","d","f","g","h","j","k","l"]),
            makeQwertyRow3(),
            makeBottomRow(leftLabel: "123", leftAction: #selector(numericTapped), includePeriod: true),
        ])
    }

    func buildNumericView() -> UIView {
        buildStandardView(rows: [
            makeSymbolRow(["1","2","3","4","5","6","7","8","9","0"]),
            makeSymbolRow(["-","/",":",";","(",")","¥","&","@","\""]),
            makeSpecialSideRow(leftLabel: "#+=", leftAction: #selector(symbolsTapped), leftWidth: 48),
            makeBottomRow(leftLabel: "ABC", leftAction: #selector(abcTapped), includePeriod: false),
        ])
    }

    func buildSymbolsView() -> UIView {
        buildStandardView(rows: [
            makeSymbolRow(["[","]","{","}","#","%","^","*","+","="]),
            makeSymbolRow(["_","\\","|","~","<",">","€","£","¥","•"]),
            makeSpecialSideRow(leftLabel: "123", leftAction: #selector(numericTapped), leftWidth: 48),
            makeBottomRow(leftLabel: "ABC", leftAction: #selector(abcTapped), includePeriod: false),
        ])
    }

    // MARK: - Shared Bottom Row

    // Used by QWERTY, the panel, and (without period) by 123 and #+=.
    func makeBottomRow(leftLabel: String, leftAction: Selector, includePeriod: Bool) -> UIStackView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fill

        let leftBtn = makeSpecialKey(leftLabel, action: leftAction)
        leftBtn.titleLabel?.font = .systemFont(ofSize: 13, weight: .medium)
        leftBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true

        // Space bar stretches to fill the remaining width.
        let spaceBtn = makeSpecialKey("space", action: #selector(spaceTapped))
        spaceBtn.titleLabel?.font = .systemFont(ofSize: 13)
        spaceBtn.setContentHuggingPriority(.init(rawValue: 1), for: .horizontal)
        spaceBtn.setContentCompressionResistancePriority(.init(rawValue: 1), for: .horizontal)

        let returnBtn = makeSpecialKey("⏎", action: #selector(returnTapped))
        returnBtn.widthAnchor.constraint(equalToConstant: 82).isActive = true

        row.addArrangedSubview(leftBtn)
        row.addArrangedSubview(spaceBtn)
        if includePeriod {
            let periodBtn = makeSpecialKey(".", action: #selector(periodTapped))
            periodBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true
            row.addArrangedSubview(periodBtn)
        }
        row.addArrangedSubview(returnBtn)
        return row
    }

    // MARK: - Row Builders

    private func makeQwertyRow3() -> UIView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fill

        let toggleBtn = makeSpecialKey("💡", action: #selector(togglePanelTapped))
        toggleBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true

        let mid = makeLetterRow(["z","x","c","v","b","n","m"])
        mid.setContentHuggingPriority(.init(rawValue: 1), for: .horizontal)

        let backBtn = makeSpecialKey("⌫", action: #selector(backspaceTapped))
        backBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true

        row.addArrangedSubview(toggleBtn)
        row.addArrangedSubview(mid)
        row.addArrangedSubview(backBtn)
        return row
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

        let mid = makeSymbolRow([".",",","?","!","'"])
        mid.setContentHuggingPriority(.init(rawValue: 1), for: .horizontal)

        let backBtn = makeSpecialKey("⌫", action: #selector(backspaceTapped))
        backBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true

        row.addArrangedSubview(leftBtn)
        row.addArrangedSubview(mid)
        row.addArrangedSubview(backBtn)
        return row
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
        let btn = UIButton(type: .system)
        btn.setTitle(letter.uppercased(), for: .normal)
        KeyStyle.applyLetter(btn)
        btn.addTarget(self, action: #selector(letterTapped(_:)), for: .touchUpInside)
        return btn
    }

    func makeSymbolKey(_ symbol: String) -> UIButton {
        let btn = UIButton(type: .system)
        btn.setTitle(symbol, for: .normal)
        KeyStyle.applySymbol(btn)
        btn.addTarget(self, action: #selector(symbolKeyTapped(_:)), for: .touchUpInside)
        return btn
    }

    func makeSpecialKey(_ title: String, action: Selector) -> UIButton {
        let btn = UIButton(type: .system)
        btn.setTitle(title, for: .normal)
        KeyStyle.applySpecial(btn)
        btn.addTarget(self, action: action, for: .touchUpInside)
        return btn
    }

    // MARK: - Private Helpers

    // Wraps an array of row views in a vertical UIStackView with standard
    // keyboard insets. All three standard layers share this container.
    private func buildStandardView(rows: [UIView]) -> UIView {
        let container = UIView()
        let stack = UIStackView(arrangedSubviews: rows)
        stack.axis = .vertical
        stack.spacing = 8
        stack.distribution = .fillEqually
        stack.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: container.topAnchor, constant: 8),
            stack.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 3),
            stack.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -3),
            stack.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -4),
        ])
        return container
    }
}
