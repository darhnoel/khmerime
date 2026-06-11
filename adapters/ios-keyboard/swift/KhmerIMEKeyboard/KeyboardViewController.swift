import UIKit

// KeyboardViewController
// ======================
// Thin UIKit shell. All input logic lives in KeyboardInputHandler.
// This class only: builds views, wires buttons → handler, and translates
// handler callbacks into UIKit view updates.
//
// Android equivalent
// ------------------
// Maps to an Android InputMethodService subclass whose onKey* methods
// delegate directly to KeyboardInputHandler.
//
// Keyboard states
// ---------------
//   .qwerty    Default roman-input view. 💡 in shift slot, 123/space/./⏎ bottom.
//   .numeric   123 layer: 1–0, punctuation, #+=, ABC/space/⏎.
//   .symbols   #+= layer: []{}#%^*+=, currencies, 123/space/⏎.
//   .panel     💡 candidate panel: chips + candidates + bottom row.
//   .charPick  CharPick mode: panel visible with A–Z chip row + candidate collection.

class KeyboardViewController: UIInputViewController {

    // MARK: - Handler

    private var handler: KeyboardInputHandler!

    // MARK: - Layout

    // Stored so viewSafeAreaInsetsDidChange can update it when the home indicator
    // appears/disappears (e.g. on iPhone X or on iPad when the bar changes).
    private var heightConstraint: NSLayoutConstraint!

    var isIPad: Bool { traitCollection.userInterfaceIdiom == .pad }
    private var baseKeyboardHeight: CGFloat { isIPad ? 320 : 260 }

    // Tag shared with KeyboardLayout so every globe button built there can be
    // shown/hidden from viewWillLayoutSubviews without a stored reference list.
    static let globeKeyTag = 999

    // MARK: - Views

    private var stripView:      StripView!
    private var panelView:      CandidatePanelView!
    private var qwertyView:     UIView!
    private var numericView:    UIView!
    private var symbolsView:    UIView!
    private var panelBottomRow: UIStackView!

    // MARK: - Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()
        handler = KeyboardInputHandler(proxy: DocumentProxyWrapper(textDocumentProxy), session: KeyboardSession())
        setupLayout()
        wireHandlerCallbacks()
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        handler.focusIn()
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        handler.focusOut()
    }

    override func viewSafeAreaInsetsDidChange() {
        super.viewSafeAreaInsetsDidChange()
        heightConstraint.constant = baseKeyboardHeight + view.safeAreaInsets.bottom
    }

    override func viewWillLayoutSubviews() {
        super.viewWillLayoutSubviews()
        let show = needsInputModeSwitchKey
        view.allDescendants(tag: Self.globeKeyTag).forEach { $0.isHidden = !show }
    }

    override func textDidChange(_ textInput: UITextInput?) {
        handler.textDidChange()
    }

    // MARK: - Handler Callbacks

    private func wireHandlerCallbacks() {
        handler.onTransition = { [weak self] state in
            self?.applyTransition(state)
        }
        handler.onRender = { [weak self] state, romanHint in
            guard let self else { return }
            self.stripView.render(state, romanBuffer: romanHint)
            switch self.handler.keyboardState {
            case .panel:
                self.panelView.render(state)
            case .charPick:
                // Only update candidates — do NOT call render() which rebuilds the
                // chip row from state.segments (empty in charPick) and destroys
                // the alphabet letter chips the user needs for their next pick.
                self.panelView.renderCharPickCandidates(state.candidates)
            default:
                break
            }
        }
        handler.onStripClear = { [weak self] in
            self?.stripView.clear()
        }
        handler.onCharPickAlphabet = { [weak self] in
            self?.panelView.renderCharPickAlphabet()
        }
    }

    // MARK: - Key Actions (forward to handler)

    @objc func nextKeyboardTapped() { advanceToNextInputMode() }

    @objc func letterTapped(_ sender: UIButton) {
        guard let ch = sender.title(for: .normal)?.lowercased(), !ch.isEmpty else { return }
        handler.sendChar(ch)
    }

    @objc func symbolKeyTapped(_ sender: UIButton) {
        guard let ch = sender.title(for: .normal), !ch.isEmpty else { return }
        handler.sendChar(ch)
    }

    @objc func periodTapped()      { handler.sendChar(".") }
    @objc func backspaceTapped()   { handler.backspaceTapped() }
    @objc func spaceTapped()       { handler.spaceTapped() }
    @objc func returnTapped()      { handler.returnTapped() }
    @objc func togglePanelTapped() { handler.togglePanel() }
    @objc func numericTapped()     { handler.switchLayer(to: .numeric) }
    @objc func symbolsTapped()     { handler.switchLayer(to: .symbols) }
    @objc func abcTapped()         { handler.switchLayer(to: .qwerty) }

    // MARK: - Strip Callbacks

    private func setupStripCallbacks() {
        stripView.onKhmerRowTapped      = { [weak self] in self?.handler.commitComposition() }
        stripView.onKhmerRowLongPressed = { [weak self] in self?.handler.togglePanel() }
    }

    // MARK: - State Machine (UIKit side)

    private func applyTransition(_ state: KeyboardState) {
        qwertyView.isHidden  = state != .qwerty
        numericView.isHidden = state != .numeric
        symbolsView.isHidden = state != .symbols
        let inPanel = state == .panel || state == .charPick
        panelView.isHidden      = !inPanel
        panelBottomRow.isHidden = !inPanel
    }

    // MARK: - Layout

    private func setupLayout() {
        // Total view height = content area + home indicator (iPhone X: 34pt, others: 0).
        // Updated in viewSafeAreaInsetsDidChange as orientation/device changes.
        view.backgroundColor = UIColor.systemGray5
        heightConstraint = view.heightAnchor.constraint(
            equalToConstant: baseKeyboardHeight + view.safeAreaInsets.bottom)
        heightConstraint.priority = UILayoutPriority(999)
        heightConstraint.isActive = true

        let root = UIView()
        root.backgroundColor = UIColor.systemGray5
        root.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(root)
        NSLayoutConstraint.activate([
            root.topAnchor.constraint(equalTo: view.topAnchor),
            root.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            root.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            // Stop at safe area so keys don't overlap the home indicator.
            root.bottomAnchor.constraint(equalTo: view.safeAreaLayoutGuide.bottomAnchor),
        ])

        stripView = StripView()
        stripView.translatesAutoresizingMaskIntoConstraints = false
        setupStripCallbacks()

        panelView = CandidatePanelView()
        panelView.delegate = self
        panelView.translatesAutoresizingMaskIntoConstraints = false

        qwertyView  = buildQwertyView()
        numericView = buildNumericView()
        symbolsView = buildSymbolsView()

        panelBottomRow = makeBottomRow(leftLabel: "123", leftAction: #selector(numericTapped), includePeriod: true)
        panelBottomRow.translatesAutoresizingMaskIntoConstraints = false

        for v in [stripView!, qwertyView!, numericView!, symbolsView!, panelView!, panelBottomRow!] {
            v.translatesAutoresizingMaskIntoConstraints = false
            root.addSubview(v)
        }

        NSLayoutConstraint.activate([
            stripView.topAnchor.constraint(equalTo: root.topAnchor),
            stripView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            stripView.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            stripView.heightAnchor.constraint(equalToConstant: 44),

            qwertyView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            qwertyView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            qwertyView.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            qwertyView.bottomAnchor.constraint(equalTo: root.bottomAnchor),

            numericView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            numericView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            numericView.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            numericView.bottomAnchor.constraint(equalTo: root.bottomAnchor),

            symbolsView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            symbolsView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            symbolsView.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            symbolsView.bottomAnchor.constraint(equalTo: root.bottomAnchor),

            panelView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            panelView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            panelView.trailingAnchor.constraint(equalTo: root.trailingAnchor),

            panelBottomRow.topAnchor.constraint(equalTo: panelView.bottomAnchorGuide.topAnchor, constant: 8),
            panelBottomRow.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 3),
            panelBottomRow.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -3),
            panelBottomRow.heightAnchor.constraint(equalToConstant: 44),
        ])

        applyTransition(.qwerty)
    }
}

// MARK: - UIView helper

private extension UIView {
    func allDescendants(tag: Int) -> [UIView] {
        var result: [UIView] = []
        for sv in subviews {
            if sv.tag == tag { result.append(sv) }
            result += sv.allDescendants(tag: tag)
        }
        return result
    }
}

// MARK: - CandidatePanelDelegate

extension KeyboardViewController: CandidatePanelDelegate {

    func candidatePanel(_ panel: CandidatePanelView, didTapChipAt index: Int) {
        handler.chipTapped(at: index)
    }

    func candidatePanel(_ panel: CandidatePanelView, didRequestEditAt index: Int) {
        handler.requestEdit(at: index)
    }

    func candidatePanelDidEnterCharPick(_ panel: CandidatePanelView) {
        handler.enterCharPickFromPanel()
    }

    func candidatePanel(_ panel: CandidatePanelView, didTapCharPickLetter letter: Character) {
        handler.charPickLetterTapped(letter)
    }

    func candidatePanel(_ panel: CandidatePanelView, didSelectCandidateAt index: Int) {
        handler.selectCandidate(at: index)
    }

    func candidatePanelDidDismiss(_ panel: CandidatePanelView) {
        handler.dismissPanel()
    }
}
