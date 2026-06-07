import UIKit

// MARK: - Keyboard State

private enum KeyboardState {
    case qwerty, numeric, symbols, panel
}

// MARK: - KeyboardViewController

class KeyboardViewController: UIInputViewController {

    // MARK: - State

    private var keyboardState: KeyboardState = .qwerty
    private var session: KhmerImeSession?
    private var romanBuffer = ""
    private var lastRenderState: IosRenderState?

    // MARK: - Strip

    private var romanLabel: UILabel!
    private var khmerLabel: UILabel!

    // MARK: - Content Views

    private var qwertyView: UIView!
    private var numericView: UIView!
    private var symbolsView: UIView!
    private var panelView: UIView!
    private var chipStack: UIStackView!
    private var candidateStack: UIStackView!

    // MARK: - Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()
        session = KhmerImeSession()
        setupLayout()
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        if let state = session?.focusIn() { render(state) }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        _ = session?.focusOut()
    }

    // MARK: - Root Layout

    private func setupLayout() {
        let h = view.heightAnchor.constraint(equalToConstant: 260)
        h.priority = UILayoutPriority(999)
        h.isActive = true

        let root = UIView()
        root.backgroundColor = UIColor.systemGray5
        root.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(root)
        NSLayoutConstraint.activate([
            root.topAnchor.constraint(equalTo: view.topAnchor),
            root.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            root.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            root.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        ])

        let strip = buildStrip()
        qwertyView  = buildQwertyView()
        numericView = buildNumericView()
        symbolsView = buildSymbolsView()
        panelView   = buildPanelView()

        for v in [strip, qwertyView!, numericView!, symbolsView!, panelView!] {
            v.translatesAutoresizingMaskIntoConstraints = false
            root.addSubview(v)
        }

        NSLayoutConstraint.activate([
            strip.topAnchor.constraint(equalTo: root.topAnchor),
            strip.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            strip.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            strip.heightAnchor.constraint(equalToConstant: 44),
        ])

        for content in [qwertyView!, numericView!, symbolsView!, panelView!] {
            NSLayoutConstraint.activate([
                content.topAnchor.constraint(equalTo: strip.bottomAnchor),
                content.leadingAnchor.constraint(equalTo: root.leadingAnchor),
                content.trailingAnchor.constraint(equalTo: root.trailingAnchor),
                content.bottomAnchor.constraint(equalTo: root.bottomAnchor),
            ])
        }

        transition(to: .qwerty)
    }

    // MARK: - State Transition

    private func transition(to state: KeyboardState) {
        keyboardState = state
        qwertyView.isHidden  = state != .qwerty
        numericView.isHidden = state != .numeric
        symbolsView.isHidden = state != .symbols
        panelView.isHidden   = state != .panel
    }

    // MARK: - Strip

    private func buildStrip() -> UIView {
        let strip = UIView()
        strip.backgroundColor = .white

        romanLabel = UILabel()
        romanLabel.font = .systemFont(ofSize: 12)
        romanLabel.textColor = .secondaryLabel
        romanLabel.textAlignment = .center
        romanLabel.translatesAutoresizingMaskIntoConstraints = false
        strip.addSubview(romanLabel)

        khmerLabel = UILabel()
        khmerLabel.font = .systemFont(ofSize: 18, weight: .medium)
        khmerLabel.textColor = .label
        khmerLabel.textAlignment = .center
        khmerLabel.translatesAutoresizingMaskIntoConstraints = false
        strip.addSubview(khmerLabel)

        let separator = UIView()
        separator.backgroundColor = UIColor.separator
        separator.translatesAutoresizingMaskIntoConstraints = false
        strip.addSubview(separator)

        NSLayoutConstraint.activate([
            romanLabel.topAnchor.constraint(equalTo: strip.topAnchor, constant: 2),
            romanLabel.leadingAnchor.constraint(equalTo: strip.leadingAnchor, constant: 8),
            romanLabel.trailingAnchor.constraint(equalTo: strip.trailingAnchor, constant: -8),
            romanLabel.heightAnchor.constraint(equalToConstant: 18),

            khmerLabel.topAnchor.constraint(equalTo: romanLabel.bottomAnchor, constant: 2),
            khmerLabel.leadingAnchor.constraint(equalTo: strip.leadingAnchor, constant: 8),
            khmerLabel.trailingAnchor.constraint(equalTo: strip.trailingAnchor, constant: -8),
            khmerLabel.bottomAnchor.constraint(equalTo: separator.topAnchor, constant: -2),

            separator.leadingAnchor.constraint(equalTo: strip.leadingAnchor),
            separator.trailingAnchor.constraint(equalTo: strip.trailingAnchor),
            separator.bottomAnchor.constraint(equalTo: strip.bottomAnchor),
            separator.heightAnchor.constraint(equalToConstant: 0.5),
        ])

        return strip
    }

    private func clearStrip() {
        romanLabel.text = ""
        khmerLabel.text = ""
    }

    // MARK: - QWERTY View

    private func buildQwertyView() -> UIView {
        let container = UIView()
        let stack = UIStackView()
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
        stack.addArrangedSubview(makeLetterRow(["q","w","e","r","t","y","u","i","o","p"]))
        stack.addArrangedSubview(makeLetterRow(["a","s","d","f","g","h","j","k","l"]))
        stack.addArrangedSubview(makeQwertyRow3())
        stack.addArrangedSubview(makeQwertyBottomRow())
        return container
    }

    private func makeQwertyRow3() -> UIView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fill

        let toggleBtn = makeSpecialKey("⊞")
        toggleBtn.addTarget(self, action: #selector(togglePanelTapped), for: .touchUpInside)
        toggleBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true

        let mid = UIStackView()
        mid.axis = .horizontal
        mid.spacing = 6
        mid.distribution = .fillEqually
        mid.setContentHuggingPriority(.init(rawValue: 1), for: .horizontal)
        for ch in ["z","x","c","v","b","n","m"] { mid.addArrangedSubview(makeLetterKey(ch)) }

        let backBtn = makeSpecialKey("⌫")
        backBtn.addTarget(self, action: #selector(backspaceTapped), for: .touchUpInside)
        backBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true

        row.addArrangedSubview(toggleBtn)
        row.addArrangedSubview(mid)
        row.addArrangedSubview(backBtn)
        return row
    }

    private func makeQwertyBottomRow() -> UIStackView {
        makeBottomRow(leftLabel: "123", leftAction: #selector(numericTapped), includePeriod: true)
    }

    // MARK: - 123 View

    private func buildNumericView() -> UIView {
        let container = UIView()
        let stack = UIStackView()
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
        stack.addArrangedSubview(makeSymbolRow(["1","2","3","4","5","6","7","8","9","0"]))
        stack.addArrangedSubview(makeSymbolRow(["-","/",":",";","(",")","¥","&","@","\""]))
        stack.addArrangedSubview(makeNumericRow3())
        stack.addArrangedSubview(makeBottomRow(leftLabel: "ABC", leftAction: #selector(abcTapped), includePeriod: false))
        return container
    }

    private func makeNumericRow3() -> UIView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fill

        let hashBtn = makeSpecialKey("#+=")
        hashBtn.titleLabel?.font = .systemFont(ofSize: 13, weight: .medium)
        hashBtn.addTarget(self, action: #selector(symbolsTapped), for: .touchUpInside)
        hashBtn.widthAnchor.constraint(equalToConstant: 48).isActive = true

        let mid = UIStackView()
        mid.axis = .horizontal
        mid.spacing = 6
        mid.distribution = .fillEqually
        mid.setContentHuggingPriority(.init(rawValue: 1), for: .horizontal)
        for ch in [".",",","?","!","'"] { mid.addArrangedSubview(makeSymbolKey(ch)) }

        let backBtn = makeSpecialKey("⌫")
        backBtn.addTarget(self, action: #selector(backspaceTapped), for: .touchUpInside)
        backBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true

        row.addArrangedSubview(hashBtn)
        row.addArrangedSubview(mid)
        row.addArrangedSubview(backBtn)
        return row
    }

    // MARK: - #+= View

    private func buildSymbolsView() -> UIView {
        let container = UIView()
        let stack = UIStackView()
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
        stack.addArrangedSubview(makeSymbolRow(["[","]","{","}","#","%","^","*","+","="]))
        stack.addArrangedSubview(makeSymbolRow(["_","\\","|","~","<",">","€","£","¥","•"]))
        stack.addArrangedSubview(makeSymbolsRow3())
        stack.addArrangedSubview(makeBottomRow(leftLabel: "ABC", leftAction: #selector(abcTapped), includePeriod: false))
        return container
    }

    private func makeSymbolsRow3() -> UIView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fill

        let numBtn = makeSpecialKey("123")
        numBtn.addTarget(self, action: #selector(numericTapped), for: .touchUpInside)
        numBtn.widthAnchor.constraint(equalToConstant: 48).isActive = true

        let mid = UIStackView()
        mid.axis = .horizontal
        mid.spacing = 6
        mid.distribution = .fillEqually
        mid.setContentHuggingPriority(.init(rawValue: 1), for: .horizontal)
        for ch in [".",",","?","!","'"] { mid.addArrangedSubview(makeSymbolKey(ch)) }

        let backBtn = makeSpecialKey("⌫")
        backBtn.addTarget(self, action: #selector(backspaceTapped), for: .touchUpInside)
        backBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true

        row.addArrangedSubview(numBtn)
        row.addArrangedSubview(mid)
        row.addArrangedSubview(backBtn)
        return row
    }

    // MARK: - Panel View

    private func buildPanelView() -> UIView {
        let container = UIView()
        container.backgroundColor = .clear

        // Chip scroll row
        let chipScroll = UIScrollView()
        chipScroll.showsHorizontalScrollIndicator = false
        chipScroll.backgroundColor = UIColor.systemBackground
        chipScroll.translatesAutoresizingMaskIntoConstraints = false

        chipStack = UIStackView()
        chipStack.axis = .horizontal
        chipStack.spacing = 8
        chipStack.alignment = .center
        chipStack.translatesAutoresizingMaskIntoConstraints = false
        chipScroll.addSubview(chipStack)
        NSLayoutConstraint.activate([
            chipStack.topAnchor.constraint(equalTo: chipScroll.topAnchor),
            chipStack.leadingAnchor.constraint(equalTo: chipScroll.leadingAnchor, constant: 8),
            chipStack.trailingAnchor.constraint(equalTo: chipScroll.trailingAnchor, constant: -8),
            chipStack.bottomAnchor.constraint(equalTo: chipScroll.bottomAnchor),
            chipStack.heightAnchor.constraint(equalTo: chipScroll.heightAnchor),
        ])

        let chipSep = UIView()
        chipSep.backgroundColor = UIColor.separator
        chipSep.translatesAutoresizingMaskIntoConstraints = false

        // Candidate scroll row
        let candScroll = UIScrollView()
        candScroll.showsHorizontalScrollIndicator = false
        candScroll.backgroundColor = UIColor.systemGray6
        candScroll.translatesAutoresizingMaskIntoConstraints = false

        candidateStack = UIStackView()
        candidateStack.axis = .horizontal
        candidateStack.spacing = 6
        candidateStack.alignment = .center
        candidateStack.translatesAutoresizingMaskIntoConstraints = false
        candScroll.addSubview(candidateStack)
        NSLayoutConstraint.activate([
            candidateStack.topAnchor.constraint(equalTo: candScroll.topAnchor),
            candidateStack.leadingAnchor.constraint(equalTo: candScroll.leadingAnchor, constant: 8),
            candidateStack.trailingAnchor.constraint(equalTo: candScroll.trailingAnchor, constant: -8),
            candidateStack.bottomAnchor.constraint(equalTo: candScroll.bottomAnchor),
            candidateStack.heightAnchor.constraint(equalTo: candScroll.heightAnchor),
        ])

        let candSep = UIView()
        candSep.backgroundColor = UIColor.separator
        candSep.translatesAutoresizingMaskIntoConstraints = false

        let bottomRow = makeQwertyBottomRow()
        bottomRow.translatesAutoresizingMaskIntoConstraints = false

        for v in [chipScroll, chipSep, candScroll, candSep, bottomRow] as [UIView] {
            container.addSubview(v)
        }

        NSLayoutConstraint.activate([
            chipScroll.topAnchor.constraint(equalTo: container.topAnchor, constant: 4),
            chipScroll.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            chipScroll.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            chipScroll.heightAnchor.constraint(equalToConstant: 44),

            chipSep.topAnchor.constraint(equalTo: chipScroll.bottomAnchor),
            chipSep.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            chipSep.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            chipSep.heightAnchor.constraint(equalToConstant: 0.5),

            candScroll.topAnchor.constraint(equalTo: chipSep.bottomAnchor),
            candScroll.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            candScroll.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            candScroll.heightAnchor.constraint(equalToConstant: 52),

            candSep.topAnchor.constraint(equalTo: candScroll.bottomAnchor),
            candSep.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            candSep.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            candSep.heightAnchor.constraint(equalToConstant: 0.5),

            bottomRow.topAnchor.constraint(equalTo: candSep.bottomAnchor, constant: 8),
            bottomRow.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 3),
            bottomRow.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -3),
            bottomRow.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -4),
        ])

        return container
    }

    private func updatePanel(_ state: IosRenderState) {
        // Chips
        chipStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        let dismissBtn = makeSpecialKey("⊞")
        dismissBtn.addTarget(self, action: #selector(togglePanelTapped), for: .touchUpInside)
        chipStack.addArrangedSubview(dismissBtn)
        for (i, seg) in state.segments.enumerated() {
            let chip = makeChipButton(text: seg.output, focused: seg.focused, index: i)
            chipStack.addArrangedSubview(chip)
        }

        // Candidates
        candidateStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        for (i, cand) in state.candidates.enumerated() {
            candidateStack.addArrangedSubview(makeCandidateButton(text: cand, index: i))
        }
    }

    // MARK: - Shared Bottom Row

    private func makeBottomRow(leftLabel: String, leftAction: Selector, includePeriod: Bool) -> UIStackView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fill

        let leftBtn = makeSpecialKey(leftLabel)
        leftBtn.titleLabel?.font = .systemFont(ofSize: 13, weight: .medium)
        leftBtn.addTarget(self, action: leftAction, for: .touchUpInside)
        leftBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true

        let spaceBtn = makeSpecialKey("space")
        spaceBtn.titleLabel?.font = .systemFont(ofSize: 13)
        spaceBtn.addTarget(self, action: #selector(spaceTapped), for: .touchUpInside)
        spaceBtn.setContentHuggingPriority(.init(rawValue: 1), for: .horizontal)
        spaceBtn.setContentCompressionResistancePriority(.init(rawValue: 1), for: .horizontal)

        let returnBtn = makeSpecialKey("⏎")
        returnBtn.addTarget(self, action: #selector(returnTapped), for: .touchUpInside)
        returnBtn.widthAnchor.constraint(equalToConstant: 82).isActive = true

        row.addArrangedSubview(leftBtn)
        row.addArrangedSubview(spaceBtn)

        if includePeriod {
            let periodBtn = makeSpecialKey(".")
            periodBtn.addTarget(self, action: #selector(periodTapped), for: .touchUpInside)
            periodBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true
            row.addArrangedSubview(periodBtn)
        }

        row.addArrangedSubview(returnBtn)
        return row
    }

    // MARK: - Key Factories

    private func makeLetterRow(_ keys: [String]) -> UIStackView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fillEqually
        for key in keys { row.addArrangedSubview(makeLetterKey(key)) }
        return row
    }

    private func makeSymbolRow(_ keys: [String]) -> UIStackView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fillEqually
        for key in keys { row.addArrangedSubview(makeSymbolKey(key)) }
        return row
    }

    private func makeLetterKey(_ letter: String) -> UIButton {
        let btn = UIButton(type: .system)
        btn.setTitle(letter.uppercased(), for: .normal)
        btn.titleLabel?.font = .systemFont(ofSize: 17)
        btn.setTitleColor(.black, for: .normal)
        applyKeyStyle(btn, white: true)
        btn.addTarget(self, action: #selector(letterTapped(_:)), for: .touchUpInside)
        return btn
    }

    private func makeSymbolKey(_ symbol: String) -> UIButton {
        let btn = UIButton(type: .system)
        btn.setTitle(symbol, for: .normal)
        btn.titleLabel?.font = .systemFont(ofSize: 17)
        btn.setTitleColor(.black, for: .normal)
        applyKeyStyle(btn, white: true)
        btn.addTarget(self, action: #selector(symbolKeyTapped(_:)), for: .touchUpInside)
        return btn
    }

    private func makeSpecialKey(_ title: String) -> UIButton {
        let btn = UIButton(type: .system)
        btn.setTitle(title, for: .normal)
        btn.titleLabel?.font = .systemFont(ofSize: 15, weight: .medium)
        btn.setTitleColor(.black, for: .normal)
        applyKeyStyle(btn, white: false)
        return btn
    }

    private func makeChipButton(text: String, focused: Bool, index: Int) -> UIView {
        let container = UIView()
        container.translatesAutoresizingMaskIntoConstraints = false

        let chip = UIButton(type: .system)
        chip.setTitle(text, for: .normal)
        chip.titleLabel?.font = .systemFont(ofSize: 16, weight: focused ? .semibold : .regular)
        chip.setTitleColor(focused ? .systemBlue : .label, for: .normal)
        chip.backgroundColor = focused ? UIColor.systemBlue.withAlphaComponent(0.12) : UIColor.systemGray5
        chip.layer.cornerRadius = 12
        chip.contentEdgeInsets = UIEdgeInsets(top: 6, left: 12, bottom: 6, right: 12)
        chip.tag = index
        chip.addTarget(self, action: #selector(chipTapped(_:)), for: .touchUpInside)
        chip.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(chip)

        // ✏ edit button — only shown on focused chip
        let editBtn = UIButton(type: .system)
        editBtn.setTitle("✏", for: .normal)
        editBtn.titleLabel?.font = .systemFont(ofSize: 12)
        editBtn.setTitleColor(.systemBlue, for: .normal)
        editBtn.tag = index
        editBtn.addTarget(self, action: #selector(editChipTapped(_:)), for: .touchUpInside)
        editBtn.translatesAutoresizingMaskIntoConstraints = false
        editBtn.isHidden = !focused
        container.addSubview(editBtn)

        NSLayoutConstraint.activate([
            chip.topAnchor.constraint(equalTo: container.topAnchor),
            chip.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            chip.bottomAnchor.constraint(equalTo: container.bottomAnchor),

            editBtn.leadingAnchor.constraint(equalTo: chip.trailingAnchor, constant: 2),
            editBtn.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            editBtn.centerYAnchor.constraint(equalTo: container.centerYAnchor),
            editBtn.widthAnchor.constraint(equalToConstant: 20),
        ])

        return container
    }

    private func makeCandidateButton(text: String, index: Int) -> UIButton {
        let btn = UIButton(type: .system)
        btn.setTitle(text, for: .normal)
        btn.titleLabel?.font = .systemFont(ofSize: 20, weight: .medium)
        btn.setTitleColor(.label, for: .normal)
        btn.backgroundColor = UIColor.white
        btn.layer.cornerRadius = 8
        btn.contentEdgeInsets = UIEdgeInsets(top: 8, left: 14, bottom: 8, right: 14)
        btn.tag = index
        btn.addTarget(self, action: #selector(candidateTapped(_:)), for: .touchUpInside)
        return btn
    }

    private func applyKeyStyle(_ btn: UIButton, white: Bool) {
        btn.backgroundColor = white ? .white : UIColor.systemGray3
        btn.layer.cornerRadius = 5
        btn.layer.shadowColor = UIColor(white: 0, alpha: 1).cgColor
        btn.layer.shadowOpacity = 0.25
        btn.layer.shadowOffset = CGSize(width: 0, height: 1)
        btn.layer.shadowRadius = 0
        btn.layer.masksToBounds = false
    }

    // MARK: - Render

    private func render(_ state: IosRenderState) {
        lastRenderState = state

        if state.segmentEditActive {
            // Segment edit mode: highlight the segment being retyped
            let editIdx = state.segmentEditIndex.map { Int($0) } ?? 0
            let parts = state.segments.enumerated().map { i, seg in
                i == editIdx ? "[\(seg.input)]" : seg.input
            }
            romanLabel.text = "✏ " + parts.joined(separator: " · ")
            khmerLabel.text = state.candidates.first ?? ""
        } else if state.segments.isEmpty {
            romanLabel.text = romanBuffer.isEmpty ? "" : romanBuffer
            khmerLabel.text = state.candidates.first ?? ""
        } else {
            romanLabel.text = state.segments.map { $0.input }.joined(separator: " · ")
            khmerLabel.text = state.segments.map { $0.output }.joined(separator: "  ")
        }

        if keyboardState == .panel {
            updatePanel(state)
        }
    }

    // MARK: - Character Input (unified)

    private func sendChar(_ ch: String) {
        guard let s = session else { return }
        textDocumentProxy.insertText(ch)
        romanBuffer += ch
        let state = s.processCharacter(ch: ch)
        // If session immediately commits (e.g. digit → Khmer digit), swap in the text field
        if let committed = state.commitText, !committed.isEmpty {
            for _ in romanBuffer { textDocumentProxy.deleteBackward() }
            textDocumentProxy.insertText(committed)
            romanBuffer = ""
        }
        render(state)
    }

    // MARK: - Key Actions

    @objc private func letterTapped(_ sender: UIButton) {
        guard let ch = sender.title(for: .normal)?.lowercased(), !ch.isEmpty else { return }
        sendChar(ch)
    }

    @objc private func symbolKeyTapped(_ sender: UIButton) {
        guard let ch = sender.title(for: .normal), !ch.isEmpty else { return }
        sendChar(ch)
    }

    @objc private func periodTapped() {
        sendChar(".")
    }

    @objc private func backspaceTapped() {
        guard let s = session else { return }
        if !romanBuffer.isEmpty { romanBuffer.removeLast() }
        textDocumentProxy.deleteBackward()
        let state = s.processBackspace()
        render(state)
        if romanBuffer.isEmpty { clearStrip() }
    }

    @objc private func spaceTapped() {
        guard let s = session else { return }
        romanBuffer += " "
        textDocumentProxy.insertText(" ")
        let state = s.processSpace()
        render(state)
    }

    @objc private func returnTapped() {
        guard let s = session else { return }
        let state = s.processEnter()

        let khmerText: String
        if !state.segments.isEmpty {
            khmerText = state.segments.map { $0.output }.joined()
        } else {
            khmerText = state.commitText ?? ""
        }

        if !khmerText.isEmpty {
            for _ in romanBuffer { textDocumentProxy.deleteBackward() }
            textDocumentProxy.insertText(khmerText)
        }

        romanBuffer = ""
        clearStrip()
        if keyboardState == .panel { transition(to: .qwerty) }
    }

    @objc private func togglePanelTapped() {
        if keyboardState == .panel {
            transition(to: .qwerty)
        } else {
            if let state = lastRenderState { updatePanel(state) }
            transition(to: .panel)
        }
    }

    @objc private func chipTapped(_ sender: UIButton) {
        guard let s = session, let current = lastRenderState else { return }
        let target = sender.tag
        let focused = current.focusedSegmentIndex.map { Int($0) } ?? 0
        let diff = target - focused
        var state = current
        if diff > 0 {
            for _ in 0..<diff  { state = s.processRight() }
        } else if diff < 0 {
            for _ in 0..<(-diff) { state = s.processLeft() }
        }
        render(state)
    }

    @objc private func editChipTapped(_ sender: UIButton) {
        guard let s = session, let current = lastRenderState else { return }
        // Navigate focus to the tapped segment, then enter Segment Edit Mode
        let target = sender.tag
        let focused = current.focusedSegmentIndex.map { Int($0) } ?? 0
        let diff = target - focused
        if diff > 0 {
            for _ in 0..<diff  { _ = s.processRight() }
        } else if diff < 0 {
            for _ in 0..<(-diff) { _ = s.processLeft() }
        }
        let state = s.processTab()
        render(state)
        // Return to QWERTY so the user can type the replacement roman input
        transition(to: .qwerty)
    }

    @objc private func candidateTapped(_ sender: UIButton) {
        guard let s = session else { return }
        let n = UInt8(min(sender.tag + 1, 9))
        let state = s.processDigit(n: n)
        render(state)
    }

    @objc private func numericTapped() { transition(to: .numeric) }
    @objc private func symbolsTapped() { transition(to: .symbols) }
    @objc private func abcTapped()     { transition(to: .qwerty) }
}
