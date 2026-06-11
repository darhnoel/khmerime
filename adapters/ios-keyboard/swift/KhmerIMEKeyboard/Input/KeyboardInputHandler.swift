import Foundation

// KeyboardInputHandler
// ====================
// Pure-Swift input logic extracted from KeyboardViewController so it can be
// unit-tested without a running UIInputViewController / textDocumentProxy.
//
// The handler owns:
//   • romanBuffer     — speculative roman chars currently in the host text field
//   • trailingSpace   — a word-separator " " inserted by spaceTapped() that is
//                       NOT in romanBuffer; consumed by returnTapped() before "\n"
//   • keyboardState   — current layer (qwerty / numeric / symbols / panel / charPick)
//   • lastState       — most recent IosRenderState from the session
//
// UI callbacks (set by KeyboardViewController in viewDidLoad):
//   onTransition      — called when keyboardState changes; VC updates view visibility
//   onRender          — called with a new state and the roman-hint string the strip
//                       should display in its top row
//   onStripClear      — called when the strip should be blanked
//   onCharPickAlphabet — called when the panel should switch to the A–Z picker
//
// Android equivalent
// ------------------
// Extract the same logic from InputMethodService into a plain Kotlin class:
//
//   class KeyboardInputHandler(
//       private val proxy: TextProxy,
//       private val session: KeyboardSession,
//   ) {
//       var onTransition: ((KeyboardState) -> Unit)? = null
//       // … same pattern
//   }

final class KeyboardInputHandler {

    let proxy: TextProxy
    let session: KeyboardSession

    // MARK: - State

    private(set) var keyboardState: KeyboardState = .qwerty
    private var romanBuffer = ""
    private var trailingSpace = false
    private(set) var lastState: IosRenderState?

    // Set after deleteBackward×N + insertText(Khmer). iOS silently appends a
    // trailing space (autocorrect replacement detection) but documentContextBeforeInput
    // is stale until UIKit calls textDidChange(), so the check must live there.
    private var pendingAutoSpaceCheck = false

    // MARK: - UI Callbacks

    var onTransition: ((KeyboardState) -> Void)?
    var onRender: ((_ state: IosRenderState, _ romanHint: String) -> Void)?
    var onStripClear: (() -> Void)?
    var onCharPickAlphabet: (() -> Void)?

    // MARK: - Init

    init(proxy: TextProxy, session: KeyboardSession) {
        self.proxy = proxy
        self.session = session
    }

    // MARK: - Lifecycle

    func focusIn() {
        let state = session.focusIn()
        render(state)
    }

    func focusOut() {
        _ = session.focusOut()
    }

    func textDidChange() {
        // Remove the iOS autocorrect auto-space appended after deleteBackward×N +
        // insertText(Khmer). documentContextBeforeInput is now fresh. A commit
        // never inserts "\n" in the same tap (⏎ with preedit commits only), so
        // the auto-space is always the last character when present.
        if pendingAutoSpaceCheck {
            pendingAutoSpaceCheck = false
            if (proxy.documentContextBeforeInput ?? "").hasSuffix(" ") {
                proxy.deleteBackward()
            }
        }

        // Detects external text changes (e.g. ✖ clear in a search bar).
        // If the text before the cursor no longer ends with our roman buffer,
        // something cleared it externally — reset composition and strip.
        guard !romanBuffer.isEmpty else { return }
        let before = proxy.documentContextBeforeInput ?? ""
        guard !before.hasSuffix(romanBuffer) else { return }
        romanBuffer = ""
        _ = session.sendReturn()
        onStripClear?()
        if keyboardState == .panel || keyboardState == .charPick {
            transition(to: .qwerty)
        }
    }

    // MARK: - Character Input

    func sendChar(_ ch: String) {
        guard keyboardState != .charPick else { return }
        trailingSpace = false
        proxy.insertText(ch)
        romanBuffer += ch
        let state = session.sendCharacter(ch)
        if let committed = state.commitText, !committed.isEmpty {
            for _ in romanBuffer { proxy.deleteBackward() }
            proxy.insertText(committed)
            romanBuffer = ""
        }
        render(state)
    }

    // MARK: - Key Actions

    func commitComposition() {
        guard keyboardState != .charPick else { return }
        let state = session.sendReturn()
        let khmerText = state.segments.isEmpty
            ? (state.commitText ?? "")
            : state.segments.map { $0.output }.joined()
        let hadRomanBuffer = !romanBuffer.isEmpty
        for _ in romanBuffer { proxy.deleteBackward() }
        if !khmerText.isEmpty {
            proxy.insertText(khmerText)
            if hadRomanBuffer {
                // iOS treats deleteBackward×N + insertText as an autocorrect
                // replacement and appends a trailing space. We can't remove it
                // synchronously because documentContextBeforeInput is stale here.
                // textDidChange() is called by UIKit once the insertion is settled
                // and the context is fresh — the check runs there.
                pendingAutoSpaceCheck = true
            }
        }
        romanBuffer = ""
        onStripClear?()
        if keyboardState == .panel { transition(to: .qwerty) }
    }

    func spaceTapped() {
        commitComposition()
        proxy.insertText(" ")
        trailingSpace = true
    }

    func returnTapped() {
        if keyboardState == .charPick {
            if let current = lastState, !current.candidates.isEmpty {
                proxy.insertText(current.candidates[0])
                _ = session.enterCharPick()
                lastState = nil
                onStripClear?()
                onCharPickAlphabet?()
                transition(to: .charPick)
            }
            return
        }
        // Standard IME contract (like Japanese/Chinese keyboards):
        //   ⏎ with active preedit  → commit only, no newline (also closes the panel)
        //   ⏎ with no preedit      → newline
        if !romanBuffer.isEmpty {
            commitComposition()
            return
        }
        if trailingSpace {
            proxy.deleteBackward()
            trailingSpace = false
        }
        proxy.insertText("\n")
    }

    func backspaceTapped() {
        trailingSpace = false
        if keyboardState == .charPick {
            if let current = lastState, !current.candidates.isEmpty {
                _ = session.enterCharPick()
                lastState = nil
                onStripClear?()
                onCharPickAlphabet?()
                transition(to: .charPick)
            } else {
                proxy.deleteBackward()
            }
            return
        }
        if !romanBuffer.isEmpty { romanBuffer.removeLast() }
        proxy.deleteBackward()
        let state = session.sendBackspace()
        render(state)
        if romanBuffer.isEmpty { onStripClear?() }
    }

    // MARK: - Panel / Layer Switches

    func togglePanel() {
        switch keyboardState {
        case .panel:
            transition(to: .qwerty)

        case .charPick:
            _ = session.exitCharPick()
            onStripClear?()
            transition(to: .qwerty)

        default:
            let hasComposition = lastState.map { !$0.candidates.isEmpty } ?? false
            if hasComposition {
                transition(to: .panel)
                if let state = lastState { onRender?(state, romanBuffer) }
            } else {
                _ = session.enterCharPick()
                lastState = nil
                onStripClear?()
                onCharPickAlphabet?()
                transition(to: .charPick)
            }
        }
    }

    func switchLayer(to state: KeyboardState) {
        transition(to: state)
    }

    // MARK: - Candidate Panel Actions

    func chipTapped(at index: Int) {
        guard let current = lastState else { return }
        let focused = current.focusedSegmentIndex.map { Int($0) } ?? 0
        let diff = index - focused
        var state = current
        if diff > 0      { for _ in 0..<diff    { state = session.sendRight() } }
        else if diff < 0 { for _ in 0..<(-diff) { state = session.sendLeft()  } }
        render(state)
    }

    func requestEdit(at index: Int) {
        guard let current = lastState else { return }
        let focused = current.focusedSegmentIndex.map { Int($0) } ?? 0
        let diff = index - focused
        if diff > 0      { for _ in 0..<diff    { _ = session.sendRight() } }
        else if diff < 0 { for _ in 0..<(-diff) { _ = session.sendLeft()  } }
        let state = session.sendTab()
        render(state)
        transition(to: .qwerty)
    }

    func enterCharPickFromPanel() {
        for _ in romanBuffer { proxy.deleteBackward() }
        romanBuffer = ""
        _ = session.enterCharPick()
        lastState = nil
        onStripClear?()
        onCharPickAlphabet?()
        transition(to: .charPick)
    }

    func charPickLetterTapped(_ letter: Character) {
        let state = session.sendCharacter(String(letter))
        lastState = state
        onRender?(state, String(letter))
    }

    func selectCandidate(at index: Int) {
        if keyboardState == .charPick {
            if let candidate = lastState?.candidates[safe: index] {
                proxy.insertText(candidate)
            }
            _ = session.enterCharPick()
            lastState = nil
            onStripClear?()
            onCharPickAlphabet?()
            return
        }
        let state = session.selectCandidate(at: index)
        render(state)
    }

    func dismissPanel() {
        if keyboardState == .charPick {
            _ = session.exitCharPick()
            onStripClear?()
        }
        transition(to: .qwerty)
    }

    // MARK: - Private

    private func render(_ state: IosRenderState) {
        lastState = state
        onRender?(state, romanBuffer)
    }

    private func transition(to newState: KeyboardState) {
        keyboardState = newState
        onTransition?(newState)
    }
}

// MARK: - Array safe subscript

private extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
