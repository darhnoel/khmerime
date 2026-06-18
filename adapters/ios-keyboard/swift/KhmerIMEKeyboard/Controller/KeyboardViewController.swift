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

    // MARK: - Handler

    private var handler: KeyboardInputHandler!

    // MARK: - Layout

    // Stored so viewSafeAreaInsetsDidChange can update it when the home indicator
    // appears/disappears (e.g. on iPhone X or on iPad when the bar changes).
    private var heightConstraint: NSLayoutConstraint!

    // Whether the strip + candidate row are currently expanded. Starts collapsed:
    // the keyboard appears keys-only and grows on the first keystroke.
    private var isChromeComposing = false

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

    private var stripView: StripView!
    private var qwertyView: UIView!
    private var numericView: UIView!
    private var symbolsView: UIView!
    private var rootView: KeyboardRootView!

    // MARK: - Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()
        Self.logMemory("launch")
        handler = KeyboardInputHandler(proxy: DocumentProxyWrapper(textDocumentProxy), session: KeyboardSession())
        Self.logMemory("after session init")
        setupLayout()
        Self.logMemory("after layout")
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
        heightConstraint.constant = keyboardHeight(composing: isChromeComposing)
    }

    // MARK: - Chrome collapse / expand

    // Total keyboard height for the current chrome state. Idle drops the strip +
    // candidate row (88pt); composing reserves them. Keys keep the same height in
    // both, so only the total shrinks.
    private func keyboardHeight(composing: Bool) -> CGFloat {
        let base = composing ? layoutMetrics.baseKeyboardHeight : layoutMetrics.idleKeyboardHeight
        return base + view.safeAreaInsets.bottom
    }

    // Expands or collapses the two chrome rows and the host height together. Only
    // acts on a real transition, so the per-keystroke renders during composition
    // don't re-trigger the animation.
    private func setComposingChrome(_ composing: Bool, animated: Bool) {
        guard composing != isChromeComposing else { return }
        isChromeComposing = composing
        rootView.setChromeVisible(composing)
        heightConstraint.constant = keyboardHeight(composing: composing)
        guard animated else { view.layoutIfNeeded(); return }
        UIView.animate(withDuration: 0.2, delay: 0, options: [.curveEaseOut, .beginFromCurrentState]) {
            self.view.layoutIfNeeded()
        }
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
            guard let self else { return }
            self.rootView.apply(state)
            let isCharPick = state == .charPick
            self.rootView.allDescendants(ofType: GlassKeyButton.self)
                .filter { $0.title(for: .normal) == "✦" }
                .forEach { $0.isGlassActive = isCharPick }
        }
        handler.onRender = { [weak self] state, romanHint in
            guard let self else { return }
            self.rootView.render(state, romanHint: romanHint)
            self.setComposingChrome(KeyboardChrome.isComposing(romanHint: romanHint, state: state), animated: true)
        }
        handler.onStripClear = { [weak self] in
            guard let self else { return }
            self.rootView.clearStrip()
            self.setComposingChrome(false, animated: true)
        }
        handler.onEnglishModeChanged = { [weak self] isEnglish in
            guard let self else { return }
            self.view.allDescendants(tag: Self.enKeyTag)
                .compactMap { $0 as? GlassKeyButton }
                .forEach { $0.isGlassActive = isEnglish }
        }
    }

    // MARK: - Key Actions (forward to handler)

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
        stripView.onSegmentFocused      = { [weak self] index in self?.handler.chipTapped(at: index) }
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
            candidateRowSelection: { [weak self] index in
                self?.handler.selectCandidate(at: index)
            }
        )

        stripView = hierarchy.stripView
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
        heightConstraint.constant = keyboardHeight(composing: false)
    }

    // MARK: - Memory diagnostics (TEMPORARY — remove after measuring)
    // Logs physical footprint and remaining headroom before iOS jetsams us at the
    // ~77 MB extension cap. Runs only at launch, never on the typing path. Read in
    // Console.app: filter by category "memory" (subsystem com.khmerime…Keyboard).

    private static let memoryLog = Logger(subsystem: "com.khmerime.KhmerIME.Keyboard", category: "memory")

    static func logMemory(_ phase: String) {
        let footprintMB = Double(currentPhysFootprintBytes()) / 1_048_576.0
        let headroomMB = Double(os_proc_available_memory()) / 1_048_576.0
        let msg = String(format: "MEM %@: footprint=%.1f MB, headroom=%.1f MB", phase, footprintMB, headroomMB)
        memoryLog.log("\(msg, privacy: .public)")
    }

    private static func currentPhysFootprintBytes() -> UInt64 {
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<integer_t>.size)
        let kr = withUnsafeMutablePointer(to: &info) { ptr in
            ptr.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count)
            }
        }
        return kr == KERN_SUCCESS ? info.phys_footprint : 0
    }

    private func wireBackspaceButtons() {
        rootView.allDescendants(ofType: BackspaceButton.self).forEach { btn in
            btn.onTap      = { [weak self] in self?.handler.backspaceTapped() }
            btn.onHoldFire = { [weak self] in self?.handler.backspaceHoldFired() }
            btn.onHoldEnd  = { [weak self] in self?.handler.backspaceHoldEnded() }
        }
    }

    // Wire every globe key (one per layer): short tap advances to the next
    // keyboard; long press shows the system keyboard picker. GlobeKeyButton
    // detects the long press internally via a timer and calls
    // handleInputModeList(from:with:) with the real touch event so the system
    // picker animates from the correct location.
    private func wireGlobeButtons() {
        rootView.allDescendants(tag: Self.globeKeyTag)
            .compactMap { $0 as? GlobeKeyButton }
            .forEach { btn in
                btn.onShortTap = { [weak self] in self?.advanceToNextInputMode() }
                btn.onLongPress = { [weak self] button, event in
                    self?.handleInputModeList(from: button, with: event)
                }
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
