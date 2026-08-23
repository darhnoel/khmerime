import UIKit
import os

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
//   .qwerty    Default roman-input view. 123 in shift slot, ✦/space/./⏎ bottom.
//   .numeric   123 layer: 1–0, punctuation, #+=, ABC/space/⏎.
//   .symbols   #+= layer: []{}#%^*+=, currencies, 123/space/⏎.
//   .charPick  CharPick mode: qwerty stays visible, ✦ highlighted, letter keys
//              browse Khmer characters without inserting roman text.

class KeyboardViewController: UIInputViewController {

    // MARK: - Temporary Diagnostics

    private static var nextDebugID = 0
    private static var activeDebugControllerCount = 0

    private let debugID = KeyboardViewController.allocateDebugID()

    private static func allocateDebugID() -> Int {
        nextDebugID += 1
        activeDebugControllerCount += 1
        return nextDebugID
    }

    // MARK: - Handler

    private var handler: KeyboardInputHandler!
    private var latestRenderState = KeyboardViewController.emptyRenderState
    private var latestRomanHint = ""

    // MARK: - Layout

    // Stored so viewSafeAreaInsetsDidChange can update it when the home indicator
    // appears/disappears (e.g. on iPhone X or on iPad when the bar changes).
    private var heightConstraint: NSLayoutConstraint!

    // Which chrome rows are currently reserved. Starts collapsed: the keyboard
    // appears keys-only and grows only when a mode has row content.
    private var chromeRows: KeyboardChrome.Rows = .none

    var isIPad: Bool { traitCollection.userInterfaceIdiom == .pad }
    var layoutMetrics: KeyboardLayoutMetrics {
        KeyboardLayoutMetrics(device: isIPad ? .pad : .phone)
    }

    // Tags shared with KeyboardLayerFactory: globe and EN occupy the same slot
    // (Option B) — exactly one is visible at a time based on needsInputModeSwitchKey.
    static let globeKeyTag = globeKeyViewTag
    static let enKeyTag    = 998

    private var layerActions: KeyboardLayerActions {
        KeyboardLayerActions(
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
    }

    // MARK: - Views

    private var stripView: StripView!
    private var candidateRowView: (UIView & KeyboardCandidateRowDisplaying)!
    private var qwertyView: UIView!
    private var numericView: UIView!
    private var symbolsView: UIView!
    private var rootView: KeyboardRootView!

    // MARK: - Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()
        setupKeyboardResources(reason: "launch")
    }

    deinit {
        Self.activeDebugControllerCount -= 1
        logMemory("deinit vc=\(debugID) active=\(Self.activeDebugControllerCount)")
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        ensureKeyboardResources()
        logMemory("viewDidAppear vc=\(debugID) active=\(Self.activeDebugControllerCount)")
        handler?.focusIn()
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        logMemory("viewWillDisappear vc=\(debugID) active=\(Self.activeDebugControllerCount)")
        handler?.focusOut()
    }

    override func viewDidDisappear(_ animated: Bool) {
        super.viewDidDisappear(animated)
        logMemory("viewDidDisappear vc=\(debugID) active=\(Self.activeDebugControllerCount)")
        teardownKeyboardResources()
        logMemory("after teardown vc=\(debugID) active=\(Self.activeDebugControllerCount)")
    }

    override func didReceiveMemoryWarning() {
        super.didReceiveMemoryWarning()
        logMemory("didReceiveMemoryWarning vc=\(debugID) active=\(Self.activeDebugControllerCount)")
    }

    override func viewSafeAreaInsetsDidChange() {
        super.viewSafeAreaInsetsDidChange()
        heightConstraint?.constant = keyboardHeight(rows: chromeRows)
    }

    // MARK: - Chrome collapse / expand

    // Total keyboard height for the current chrome state. Keys keep the same
    // height in every state; only the chrome rows add height above them.
    private func keyboardHeight(rows: KeyboardChrome.Rows) -> CGFloat {
        let chromeHeight: CGFloat
        switch rows {
        case .none:
            chromeHeight = 0
        case .stripOnly:
            chromeHeight = layoutMetrics.stripHeight
        case .candidateOnly:
            chromeHeight = layoutMetrics.candidateRowHeight
        case .stripAndCandidate:
            chromeHeight = layoutMetrics.stripHeight + layoutMetrics.candidateRowHeight
        }
        return layoutMetrics.idleKeyboardHeight + chromeHeight + view.safeAreaInsets.bottom
    }

    // Applies row constraints and host height together. Only acts on a real
    // transition, so per-keystroke renders do not re-trigger the animation.
    private func setChromeRows(_ rows: KeyboardChrome.Rows, animated: Bool) {
        guard rows != chromeRows else { return }
        chromeRows = rows
        guard let rootView, let heightConstraint else { return }
        rootView.setChromeRows(rows)
        heightConstraint.constant = keyboardHeight(rows: rows)
        guard animated else { view.layoutIfNeeded(); return }
        UIView.animate(withDuration: 0.2, delay: 0, options: [.curveEaseOut, .beginFromCurrentState]) {
            self.view.layoutIfNeeded()
        }
    }

    override func viewWillLayoutSubviews() {
        super.viewWillLayoutSubviews()
        // The globe is ALWAYS visible (ADR-0022): gating it on
        // needsInputModeSwitchKey hid it on iPad / iPhone X-class devices and
        // got the app rejected under Guideline 4.4.1. EN stays visible too.
        let visibility = SwitchKeyVisibility(needsInputModeSwitchKey: needsInputModeSwitchKey)
        view.allDescendants(tag: Self.globeKeyTag).forEach { $0.isHidden = visibility.globeHidden }
        view.allDescendants(tag: Self.enKeyTag).forEach { $0.isHidden = visibility.englishHidden }
    }

    override func textDidChange(_ textInput: UITextInput?) {
        handler?.textDidChange()
    }

    // TEMPORARY: device-only memory probe for diagnosing System Khmer Fallback.
    // Remove after the current jetsam investigation is resolved.
    private func logMemory(_ label: String) {
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.stride / MemoryLayout<integer_t>.stride)
        let result = withUnsafeMutablePointer(to: &info) { pointer in
            pointer.withMemoryRebound(to: integer_t.self, capacity: Int(count)) { rebound in
                task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), rebound, &count)
            }
        }
        guard result == KERN_SUCCESS else {
            NSLog("MEM %@ unavailable kern_return=%d", label, result)
            return
        }

        let footprintMB = Double(info.phys_footprint) / 1_048_576.0
        let availableMB = Double(os_proc_available_memory()) / 1_048_576.0
        NSLog("MEM %@ footprint=%.1f MB headroom=%.1f MB", label, footprintMB, availableMB)
    }

    // MARK: - Handler Callbacks

    private func setupKeyboardResources(reason: String) {
        logMemory("\(reason) vc=\(debugID) active=\(Self.activeDebugControllerCount)")
        let session = KeyboardSession()
        // Register an optional provider (no-op in the OSS build), then honor the saved Standard/Smart
        // choice (shared App Group). Inert without a registered provider, so the OSS build stays Standard.
        AiModelArming.armIfNeeded()
        session.setModelMode(SmartModePreference().isEnabled)
        logMemory("after KeyboardSession() vc=\(debugID) active=\(Self.activeDebugControllerCount)")
        handler = KeyboardInputHandler(proxy: DocumentProxyWrapper(textDocumentProxy), session: session)
        setupLayout()
        logMemory("after layout vc=\(debugID) active=\(Self.activeDebugControllerCount)")
        wireHandlerCallbacks()
    }

    private func ensureKeyboardResources() {
        guard handler == nil || rootView == nil else { return }
        setupKeyboardResources(reason: "rebuild")
    }

    private func teardownKeyboardResources() {
        guard handler != nil || rootView != nil else { return }

        handler?.onTransition = nil
        handler?.onRender = nil
        handler?.onStripClear = nil
        handler?.onEnglishModeChanged = nil

        if let rootView {
            KeyboardResourceTeardown.releaseInteractions(in: rootView)
            rootView.removeFromSuperview()
        }

        heightConstraint?.isActive = false
        handler = nil
        heightConstraint = nil
        stripView = nil
        candidateRowView = nil
        qwertyView = nil
        numericView = nil
        symbolsView = nil
        rootView = nil
        chromeRows = .none
        latestRenderState = Self.emptyRenderState
        latestRomanHint = ""
    }

    private func wireHandlerCallbacks() {
        handler.onTransition = { [weak self] state in
            guard let self else { return }
            self.rootView?.apply(state)
            let isCharPick = state == .charPick
            self.rootView?.allDescendants(ofType: GlassKeyButton.self)
                .filter { $0.title(for: .normal) == "✦" }
                .forEach { $0.isGlassActive = isCharPick }
            self.renderChrome(state: self.latestRenderState, romanHint: self.latestRomanHint)
        }
        handler.onRender = { [weak self] state, romanHint in
            guard let self else { return }
            self.latestRenderState = state
            self.latestRomanHint = romanHint
            self.renderChrome(state: state, romanHint: romanHint)
        }
        handler.onStripClear = { [weak self] in
            guard let self else { return }
            self.latestRenderState = Self.emptyRenderState
            self.latestRomanHint = ""
            self.renderChrome(state: Self.emptyRenderState, romanHint: "")
        }
        handler.onEnglishModeChanged = { [weak self] isEnglish in
            guard let self else { return }
            self.view.allDescendants(tag: Self.enKeyTag)
                .compactMap { $0 as? GlassKeyButton }
                .forEach { $0.isGlassActive = isEnglish }
            self.renderChrome(state: Self.emptyRenderState, romanHint: "")
        }
    }

    private func renderChrome(state: IosRenderState, romanHint: String) {
        let keyboardState = handler.keyboardState
        let presentation = KeyboardChrome.presentation(
            isEnglish: handler.isEnglishMode,
            keyboardState: keyboardState,
            romanHint: romanHint,
            state: state
        )
        setChromeRows(presentation.rows, animated: true)
        switch presentation {
        case .hidden:
            rootView?.clearStrip()
        case .quickAccess:
            rootView?.showQuickAccess(charPickOnly: false) { [weak self] item in
                self?.handler?.insertQuickAccess(item.commitText)
            }
        case .charPickQuickAccess:
            rootView?.showQuickAccess(charPickOnly: true) { [weak self] item in
                self?.handler?.insertQuickAccess(item.commitText)
            }
        case .composition, .charPickCandidates:
            rootView?.render(state, romanHint: romanHint, keyboardState: keyboardState)
        }
    }

    private static let emptyRenderState = IosRenderState(
        candidates: [], selectedIndex: nil, preedit: "", segments: [],
        focusedSegmentIndex: nil, commitText: nil, segmentEditActive: false,
        segmentEditIndex: nil, phraseCandidates: [], selectedPhraseIndex: 0
    )

    // MARK: - Key Actions (forward to handler)

    @objc func toggleEnglishTapped()   { handler?.toggleEnglish() }

    @objc func letterTapped(_ sender: UIButton) {
        guard let ch = sender.title(for: .normal)?.lowercased(), !ch.isEmpty else { return }
        handler?.sendChar(ch)
    }

    @objc func literalKeyTapped(_ sender: UIButton) {
        guard let ch = sender.title(for: .normal), !ch.isEmpty else { return }
        handler?.sendLiteralKeycap(ch)
    }

    @objc func backspaceTapped()   { handler?.backspaceTapped() }
    @objc func spaceTapped()       { handler?.spaceTapped() }
    @objc func returnTapped()      { handler?.returnTapped() }
    @objc func togglePanelTapped() { handler?.togglePanel() }
    @objc func numericTapped()     { handler?.switchLayer(to: .numeric) }
    @objc func symbolsTapped()     { handler?.switchLayer(to: .symbols) }
    @objc func abcTapped()         { handler?.switchLayer(to: .qwerty) }

    // MARK: - Strip Callbacks

    private func setupStripCallbacks() {
        stripView.onKhmerRowTapped      = { [weak self] in self?.handler?.commitComposition() }
        stripView.onKhmerRowLongPressed = { [weak self] in self?.handler?.togglePanel() }
        stripView.onSegmentFocused      = { [weak self] index in self?.handler?.chipTapped(at: index) }
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
        ).build(
            candidateSelection: { [weak self] index in
                self?.handler?.selectCandidate(at: index)
            },
            phraseSelection: { [weak self] index in
                self?.handler?.selectPhrase(at: index)
            }
        )

        stripView = hierarchy.stripView
        candidateRowView = hierarchy.candidateRowView
        qwertyView = hierarchy.qwertyView
        numericView = hierarchy.numericView
        symbolsView = hierarchy.symbolsView
        rootView = hierarchy.rootView

        setupStripCallbacks()
        wireBackspaceButtons()
        wireGlobeButtons()
        heightConstraint = KeyboardHostLayout.install(
            rootView: rootView,
            in: view,
            metrics: layoutMetrics,
            safeAreaBottom: view.safeAreaInsets.bottom
        )
        // rootView starts with its chrome collapsed, so the host begins at idle
        // height — keys-only — and expands on the first keystroke.
        heightConstraint.constant = keyboardHeight(rows: .none)
    }

    private func wireBackspaceButtons() {
        rootView.allDescendants(ofType: BackspaceButton.self).forEach { btn in
            btn.onTap      = { [weak self] in self?.handler?.backspaceTapped() }
            btn.onHoldFire = { [weak self] in self?.handler?.backspaceHoldFired() }
            btn.onHoldEnd  = { [weak self] in self?.handler?.backspaceHoldEnded() }
        }
    }

    // Wire every globe key (one per layer) to UIKit's built-in next-keyboard
    // handling: handleInputModeList(from:with:) on .allTouchEvents. UIKit calls
    // it with a LIVE event and itself decides tap vs long-press — a tap advances
    // to the next keyboard, a long press shows the system picker anchored at the
    // button. (A custom timer that stashed the touchesBegan event failed: a
    // UIEvent captured then is stale 0.5s later, so the picker never appeared.)
    private func wireGlobeButtons() {
        rootView.allDescendants(tag: Self.globeKeyTag)
            .compactMap { $0 as? UIButton }
            .forEach { btn in
                btn.removeTarget(nil, action: nil, for: .allTouchEvents)
                btn.addTarget(self, action: #selector(handleInputModeList(from:with:)), for: .allTouchEvents)
            }
    }
}

// MARK: - Key-press feedback

// The controller's own view IS the extension's UIInputView. Conforming here (not on
// KeyboardRootView, which is a plain subview) is what lets UIDevice.playInputClick()
// — called from GlassKeyButton.touchesBegan — actually emit the user's configured
// keyboard sound/haptic, WITHOUT changing the custom layout. Requires Full Access and
// the user's keyboard-feedback setting; otherwise it silently no-ops.
extension KeyboardViewController: UIInputViewAudioFeedback {
    var enableInputClicksWhenVisible: Bool { true }
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
