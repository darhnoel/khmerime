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
//   .qwerty    Default roman-input view. ✦ in shift slot, 123/space/./⏎ bottom.
//   .numeric   123 layer: 1–0, punctuation, #+=, ABC/space/⏎.
//   .symbols   #+= layer: []{}#%^*+=, currencies, 123/space/⏎.
//   .panel     ✦ candidate panel: chips + candidates + bottom row.
//   .charPick  CharPick mode: panel visible with A–Z chip row + candidate collection.

class KeyboardViewController: UIInputViewController {

    // MARK: - Handler

    private var handler: KeyboardInputHandler!

    // MARK: - Layout

    // Stored so viewSafeAreaInsetsDidChange can update it when the home indicator
    // appears/disappears (e.g. on iPhone X or on iPad when the bar changes).
    private var heightConstraint: NSLayoutConstraint!

    var isIPad: Bool { traitCollection.userInterfaceIdiom == .pad }
    var layoutMetrics: KeyboardLayoutMetrics {
        KeyboardLayoutMetrics(device: isIPad ? .pad : .phone)
    }

    // Tags shared with KeyboardLayerFactory: globe and EN occupy the same slot
    // (Option B) — exactly one is visible at a time based on needsInputModeSwitchKey.
    static let globeKeyTag = 999
    static let enKeyTag    = 998

    private var layerActions: KeyboardLayerActions {
        KeyboardLayerActions(
            nextKeyboard: #selector(nextKeyboardTapped),
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
    }

    // MARK: - Views

    private var stripView:      StripView!
    private var panelView:      CandidatePanelView!
    private var qwertyView:     UIView!
    private var numericView:    UIView!
    private var symbolsView:    UIView!
    private var panelBottomRow: UIStackView!
    private var rootView:       KeyboardRootView!

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
        heightConstraint.constant = KeyboardHostLayout.heightConstant(
            metrics: layoutMetrics,
            safeAreaBottom: view.safeAreaInsets.bottom
        )
    }

    override func viewWillLayoutSubviews() {
        super.viewWillLayoutSubviews()
        let show = needsInputModeSwitchKey
        view.allDescendants(tag: Self.globeKeyTag).forEach { $0.isHidden = !show }
        view.allDescendants(tag: Self.enKeyTag).forEach { $0.isHidden = show }
    }

    override func textDidChange(_ textInput: UITextInput?) {
        handler.textDidChange()
    }

    // MARK: - Handler Callbacks

    private func wireHandlerCallbacks() {
        handler.onTransition = { [weak self] state in
            self?.rootView.apply(state)
        }
        handler.onRender = { [weak self] state, romanHint in
            guard let self else { return }
            self.rootView.render(state, romanHint: romanHint, keyboardState: self.handler.keyboardState)
        }
        handler.onStripClear = { [weak self] in
            self?.rootView.clearStrip()
        }
        handler.onCharPickAlphabet = { [weak self] in
            self?.rootView.renderCharPickAlphabet()
        }
        handler.onEnglishModeChanged = { [weak self] isEnglish in
            guard let self else { return }
            self.view.allDescendants(tag: Self.enKeyTag)
                .compactMap { $0 as? GlassKeyButton }
                .forEach { $0.isGlassActive = isEnglish }
        }
    }

    // MARK: - Key Actions (forward to handler)

    @objc func nextKeyboardTapped()    { advanceToNextInputMode() }
    @objc func toggleEnglishTapped()   { handler.toggleEnglish() }

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

    // MARK: - Layout

    private func setupLayout() {
        let hierarchy = KeyboardViewHierarchyBuilder(
            metrics: layoutMetrics,
            isIPad: isIPad,
            target: self,
            globeKeyTag: Self.globeKeyTag,
            enKeyTag: Self.enKeyTag,
            actions: layerActions
        ).build(panelDelegate: self)

        stripView = hierarchy.stripView
        panelView = hierarchy.panelView
        qwertyView = hierarchy.qwertyView
        numericView = hierarchy.numericView
        symbolsView = hierarchy.symbolsView
        panelBottomRow = hierarchy.panelBottomRow
        rootView = hierarchy.rootView

        setupStripCallbacks()
        wireBackspaceButtons()
        heightConstraint = KeyboardHostLayout.install(
            rootView: rootView,
            in: view,
            metrics: layoutMetrics,
            safeAreaBottom: view.safeAreaInsets.bottom
        )
    }

    private func wireBackspaceButtons() {
        rootView.allDescendants(ofType: BackspaceButton.self).forEach { btn in
            btn.onTap      = { [weak self] in self?.handler.backspaceTapped() }
            btn.onHoldFire = { [weak self] in self?.handler.backspaceHoldFired() }
            btn.onHoldEnd  = { [weak self] in self?.handler.backspaceHoldEnded() }
        }
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

    func allDescendants<T: UIView>(ofType _: T.Type) -> [T] {
        var result: [T] = []
        for sv in subviews {
            if let typed = sv as? T { result.append(typed) }
            result += sv.allDescendants(ofType: T.self)
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
