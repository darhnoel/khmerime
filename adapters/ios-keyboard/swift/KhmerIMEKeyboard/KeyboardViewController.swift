import UIKit

class KeyboardViewController: UIInputViewController {

    private var session: KhmerImeSession?

    // Strip labels
    private var romanLabel: UILabel!   // nhom · ttov · salarien
    private var khmerLabel: UILabel!   // ខ្ញុំ  ទៅ  សាលារៀន

    // Roman buffer — tracks what's been inserted into the text field so far.
    // Used to display the strip and to delete-and-replace on Enter.
    private var romanBuffer = ""

    override func viewDidLoad() {
        super.viewDidLoad()
        session = KhmerImeSession()
        setupKeyboard()
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        if let state = session?.focusIn() { render(state) }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        _ = session?.focusOut()
    }

    // MARK: - Render

    private func render(_ state: IosRenderState) {
        if state.segments.isEmpty {
            // No segmentation yet — show raw roman and top candidate
            romanLabel.text = romanBuffer
            khmerLabel.text = state.candidates.first ?? ""
        } else {
            // Segmented session — show roman slices and Khmer per segment
            let romanParts = state.segments.map { $0.input }
            romanLabel.text = romanParts.joined(separator: " · ")
            let khmerParts = state.segments.map { $0.output }
            khmerLabel.text = khmerParts.joined(separator: "  ")
        }
    }

    private func clearStrip() {
        romanLabel.text = ""
        khmerLabel.text = ""
    }

    // MARK: - Layout

    private func setupKeyboard() {
        let h = view.heightAnchor.constraint(equalToConstant: 250)
        h.priority = UILayoutPriority(999)
        h.isActive = true

        let keyboard = UIView()
        keyboard.backgroundColor = UIColor.systemGray5
        keyboard.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(keyboard)
        NSLayoutConstraint.activate([
            keyboard.topAnchor.constraint(equalTo: view.topAnchor),
            keyboard.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            keyboard.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            keyboard.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        ])

        let strip = buildStrip()
        let keysStack = buildKeysStack()
        keyboard.addSubview(strip)
        keyboard.addSubview(keysStack)

        NSLayoutConstraint.activate([
            strip.topAnchor.constraint(equalTo: keyboard.topAnchor),
            strip.leadingAnchor.constraint(equalTo: keyboard.leadingAnchor),
            strip.trailingAnchor.constraint(equalTo: keyboard.trailingAnchor),
            strip.heightAnchor.constraint(equalToConstant: 44),

            keysStack.topAnchor.constraint(equalTo: strip.bottomAnchor, constant: 4),
            keysStack.leadingAnchor.constraint(equalTo: keyboard.leadingAnchor, constant: 3),
            keysStack.trailingAnchor.constraint(equalTo: keyboard.trailingAnchor, constant: -3),
            keysStack.bottomAnchor.constraint(equalTo: keyboard.bottomAnchor, constant: -4),
        ])
    }

    private func buildStrip() -> UIView {
        let strip = UIView()
        strip.backgroundColor = .white
        strip.translatesAutoresizingMaskIntoConstraints = false

        let separator = UIView()
        separator.backgroundColor = UIColor.separator
        separator.translatesAutoresizingMaskIntoConstraints = false
        strip.addSubview(separator)

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

    private func buildKeysStack() -> UIStackView {
        let outer = UIStackView()
        outer.axis = .vertical
        outer.spacing = 8
        outer.distribution = .fillEqually
        outer.translatesAutoresizingMaskIntoConstraints = false

        outer.addArrangedSubview(makeLetterRow(["q","w","e","r","t","y","u","i","o","p"]))
        outer.addArrangedSubview(makeLetterRow(["a","s","d","f","g","h","j","k","l"]))
        outer.addArrangedSubview(makeLetterRow(["z","x","c","v","b","n","m","⌫"]))
        outer.addArrangedSubview(makeBottomRow())

        return outer
    }

    private func makeLetterRow(_ keys: [String]) -> UIStackView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fillEqually
        for key in keys {
            if key == "⌫" {
                let btn = makeSpecialKey("⌫")
                btn.addTarget(self, action: #selector(backspaceTapped), for: .touchUpInside)
                row.addArrangedSubview(btn)
            } else {
                row.addArrangedSubview(makeLetterKey(key))
            }
        }
        return row
    }

    private func makeBottomRow() -> UIStackView {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 6
        row.distribution = .fill

        // Toggle button — switches to candidate panel view (not yet implemented)
        let toggleBtn = makeSpecialKey("⊞")
        toggleBtn.addTarget(self, action: #selector(togglePanelTapped), for: .touchUpInside)
        toggleBtn.widthAnchor.constraint(equalToConstant: 42).isActive = true

        let spaceBtn = makeSpecialKey("space")
        spaceBtn.titleLabel?.font = .systemFont(ofSize: 13)
        spaceBtn.addTarget(self, action: #selector(spaceTapped), for: .touchUpInside)
        spaceBtn.setContentHuggingPriority(.init(rawValue: 1), for: .horizontal)
        spaceBtn.setContentCompressionResistancePriority(.init(rawValue: 1), for: .horizontal)

        let returnBtn = makeSpecialKey("return")
        returnBtn.titleLabel?.font = .systemFont(ofSize: 13)
        returnBtn.addTarget(self, action: #selector(returnTapped), for: .touchUpInside)
        returnBtn.widthAnchor.constraint(equalToConstant: 82).isActive = true

        row.addArrangedSubview(toggleBtn)
        row.addArrangedSubview(spaceBtn)
        row.addArrangedSubview(returnBtn)
        return row
    }

    // MARK: - Key factories

    private func makeLetterKey(_ letter: String) -> UIButton {
        let btn = UIButton(type: .system)
        btn.setTitle(letter.uppercased(), for: .normal)
        applyKeyStyle(btn, white: true)
        btn.titleLabel?.font = .systemFont(ofSize: 17)
        btn.setTitleColor(.black, for: .normal)
        btn.addTarget(self, action: #selector(letterTapped(_:)), for: .touchUpInside)
        return btn
    }

    private func makeSpecialKey(_ title: String) -> UIButton {
        let btn = UIButton(type: .system)
        btn.setTitle(title, for: .normal)
        applyKeyStyle(btn, white: false)
        btn.titleLabel?.font = .systemFont(ofSize: 15, weight: .medium)
        btn.setTitleColor(.black, for: .normal)
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

    // MARK: - Key actions

    @objc private func letterTapped(_ sender: UIButton) {
        guard let ch = sender.title(for: .normal)?.lowercased(), !ch.isEmpty, let s = session else { return }
        romanBuffer += ch
        textDocumentProxy.insertText(ch)
        let state = s.processCharacter(ch: ch)
        render(state)
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

        // Build the Khmer commit text from segments (concatenated, no spaces)
        let khmerText: String
        if !state.segments.isEmpty {
            khmerText = state.segments.map { $0.output }.joined()
        } else {
            khmerText = state.commitText ?? ""
        }

        // Replace roman buffer in the text field with Khmer
        if !khmerText.isEmpty {
            for _ in romanBuffer { textDocumentProxy.deleteBackward() }
            textDocumentProxy.insertText(khmerText)
        }

        romanBuffer = ""
        clearStrip()
    }

    @objc private func togglePanelTapped() {
        // TODO: switch to candidate panel view
    }
}
