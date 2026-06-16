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
//   • dispatcher      — controls where session calls execute (background queue in
//                       production, inline in tests). See KeyboardDispatcher.swift.
//
// Dispatch contract
// -----------------
// For letter keys (sendChar): proxy.insertText is immediate on the calling thread.
// The session call and render are deferred through the dispatcher. Letters never
// produce commitText, so no proxy mutation happens in the deferred block.
//
// For commit-path operations (space / return / commitComposition): roman chars are
// deleted immediately (count is known from romanBuffer). The Khmer commit text is
// inserted inside the onMain block, after the session has resolved it. This
// preserves correct ordering even when those operations are deferred.
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
    private let dispatcher: KeyboardDispatcher

    // MARK: - State

    private(set) var keyboardState: KeyboardState = .qwerty
    private(set) var isEnglishMode = false
    private var romanBuffer = ""
    private var trailingSpace = false
    private(set) var lastState: IosRenderState?

    // Counts how many roman-buffer chars were deleted by backspaceHoldFired()
    // without a matching session call. backspaceHoldEnded() drains this with
    // one batched session block, then resets to 0.
    private var pendingHoldBackspaces = 0

    // Incremented on every sendChar(). A keystroke's render is only applied
    // if its captured generation still matches when its onMain block runs —
    // otherwise a newer keystroke has already superseded it, so the stale
    // render is skipped to keep the main thread from queuing up backlog.
    private var sendGeneration = 0

    // Set after deleteBackward×N + insertText(Khmer). iOS silently appends a
    // trailing space (autocorrect replacement detection) but documentContextBeforeInput
    // is stale until UIKit calls textDidChange(), so the check must live there.
    private var pendingAutoSpaceCheck = false

    // MARK: - UI Callbacks

    var onTransition: ((KeyboardState) -> Void)?
    var onRender: ((_ state: IosRenderState, _ romanHint: String) -> Void)?
    var onStripClear: (() -> Void)?
    var onCharPickAlphabet: (() -> Void)?
    var onEnglishModeChanged: ((Bool) -> Void)?

    // MARK: - Init

    init(proxy: TextProxy, session: KeyboardSession, dispatcher: KeyboardDispatcher = QueuedDispatcher()) {
        self.proxy = proxy
        self.session = session
        self.dispatcher = dispatcher
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
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            _ = self.session.sendReturn()
            self.dispatcher.onMain { [weak self] in
                guard let self else { return }
                self.onStripClear?()
                if self.keyboardState == .panel || self.keyboardState == .charPick {
                    self.transition(to: .qwerty)
                }
            }
        }
    }

    // MARK: - English Mode Toggle

    func toggleEnglish() {
        if isEnglishMode {
            isEnglishMode = false
        } else {
            dispatcher.onSession { [weak self] in
                guard let self else { return }
                _ = self.session.sendReturn()
                self.dispatcher.onMain { [weak self] in
                    guard let self else { return }
                    self.romanBuffer = ""
                    self.trailingSpace = false
                    self.onStripClear?()
                }
            }
            isEnglishMode = true
        }
        onEnglishModeChanged?(isEnglishMode)
    }

    // MARK: - Character Input

    func sendChar(_ ch: String) {
        guard keyboardState != .charPick else { return }
        if isEnglishMode {
            proxy.insertText(ch)
            return
        }
        trailingSpace = false
        // Proxy insertion is immediate — the typed char appears in the text field
        // before the session block executes (critical for responsiveness).
        proxy.insertText(ch)
        romanBuffer += ch
        sendGeneration += 1
        let myGeneration = sendGeneration
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            let state = self.session.sendCharacter(ch)
            self.dispatcher.onMain { [weak self] in
                guard let self else { return }
                // Letters never produce commitText; this guard is a safety net
                // for any symbol that might trigger a single-keycap auto-commit.
                if let committed = state.commitText, !committed.isEmpty {
                    for _ in self.romanBuffer { self.proxy.deleteBackward() }
                    self.proxy.insertText(committed)
                    self.romanBuffer = ""
                }
                // A newer keystroke has already been dispatched — its render
                // supersedes this one, so skip to avoid a stale UI update.
                guard myGeneration == self.sendGeneration else { return }
                self.render(state)
            }
        }
    }

    // MARK: - Key Actions

    func commitComposition() {
        guard keyboardState != .charPick else { return }
        // Delete the roman buffer immediately — we know the count now.
        let hadRomanBuffer = !romanBuffer.isEmpty
        for _ in romanBuffer { proxy.deleteBackward() }
        romanBuffer = ""
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            let state = self.session.sendReturn()
            self.dispatcher.onMain { [weak self] in
                guard let self else { return }
                let khmerText = state.segments.isEmpty
                    ? (state.commitText ?? "")
                    : state.segments.map { $0.output }.joined()
                if !khmerText.isEmpty {
                    self.proxy.insertText(khmerText)
                    if hadRomanBuffer {
                        // iOS treats deleteBackward×N + insertText as an autocorrect
                        // replacement and appends a trailing space. We can't remove it
                        // synchronously because documentContextBeforeInput is stale here.
                        // textDidChange() is called by UIKit once the insertion is settled
                        // and the context is fresh — the check runs there.
                        self.pendingAutoSpaceCheck = true
                    }
                }
                self.onStripClear?()
                if self.keyboardState == .panel { self.transition(to: .qwerty) }
            }
        }
    }

    func spaceTapped() {
        if isEnglishMode {
            proxy.insertText(" ")
            return
        }
        // Delete roman immediately, then let session resolve the Khmer commit.
        // Space is appended inside onMain so it always follows the committed Khmer.
        let hadRomanBuffer = !romanBuffer.isEmpty
        for _ in romanBuffer { proxy.deleteBackward() }
        romanBuffer = ""
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            let state = self.session.sendReturn()
            self.dispatcher.onMain { [weak self] in
                guard let self else { return }
                let khmerText = state.segments.isEmpty
                    ? (state.commitText ?? "")
                    : state.segments.map { $0.output }.joined()
                if !khmerText.isEmpty {
                    self.proxy.insertText(khmerText)
                    if hadRomanBuffer { self.pendingAutoSpaceCheck = true }
                }
                self.proxy.insertText(" ")
                self.trailingSpace = true
                self.onStripClear?()
                if self.keyboardState == .panel { self.transition(to: .qwerty) }
            }
        }
    }

    func returnTapped() {
        if isEnglishMode {
            proxy.insertText("\n")
            return
        }
        if keyboardState == .charPick {
            if let current = lastState, !current.candidates.isEmpty {
                proxy.insertText(current.candidates[0])
                dispatcher.onSession { [weak self] in
                    guard let self else { return }
                    _ = self.session.enterCharPick()
                    self.dispatcher.onMain { [weak self] in
                        guard let self else { return }
                        self.lastState = nil
                        self.onStripClear?()
                        self.onCharPickAlphabet?()
                        self.transition(to: .charPick)
                    }
                }
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

    func backspaceHoldFired() {
        if isEnglishMode { proxy.deleteBackward(); return }
        trailingSpace = false
        if keyboardState == .charPick { proxy.deleteBackward(); return }
        if !romanBuffer.isEmpty {
            romanBuffer.removeLast()
            pendingHoldBackspaces += 1
        }
        proxy.deleteBackward()
        // No session dispatch — backspaceHoldEnded() batches them all at once.
    }

    func backspaceHoldEnded() {
        let count = pendingHoldBackspaces
        pendingHoldBackspaces = 0
        guard count > 0 else { return }
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            var state: IosRenderState?
            for _ in 0..<count { state = self.session.sendBackspace() }
            guard let finalState = state else { return }
            self.dispatcher.onMain { [weak self] in
                guard let self else { return }
                self.render(finalState)
                if self.romanBuffer.isEmpty { self.onStripClear?() }
            }
        }
    }

    func backspaceTapped() {
        if isEnglishMode {
            proxy.deleteBackward()
            return
        }
        trailingSpace = false
        if keyboardState == .charPick {
            if let current = lastState, !current.candidates.isEmpty {
                dispatcher.onSession { [weak self] in
                    guard let self else { return }
                    _ = self.session.enterCharPick()
                    self.dispatcher.onMain { [weak self] in
                        guard let self else { return }
                        self.lastState = nil
                        self.onStripClear?()
                        self.onCharPickAlphabet?()
                        self.transition(to: .charPick)
                    }
                }
            } else {
                proxy.deleteBackward()
            }
            return
        }
        if !romanBuffer.isEmpty { romanBuffer.removeLast() }
        proxy.deleteBackward()
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            let state = self.session.sendBackspace()
            self.dispatcher.onMain { [weak self] in
                guard let self else { return }
                self.render(state)
                if self.romanBuffer.isEmpty { self.onStripClear?() }
            }
        }
    }

    // MARK: - Panel / Layer Switches

    func togglePanel() {
        if isEnglishMode {
            isEnglishMode = false
            onEnglishModeChanged?(false)
        }
        switch keyboardState {
        case .panel:
            transition(to: .qwerty)

        case .charPick:
            dispatcher.onSession { [weak self] in
                guard let self else { return }
                _ = self.session.exitCharPick()
                self.dispatcher.onMain { [weak self] in
                    guard let self else { return }
                    self.onStripClear?()
                    self.transition(to: .qwerty)
                }
            }

        default:
            let hasComposition = lastState.map { !$0.candidates.isEmpty } ?? false
            if hasComposition {
                transition(to: .panel)
                if let state = lastState { onRender?(state, romanBuffer) }
            } else {
                dispatcher.onSession { [weak self] in
                    guard let self else { return }
                    _ = self.session.enterCharPick()
                    self.dispatcher.onMain { [weak self] in
                        guard let self else { return }
                        self.lastState = nil
                        self.onStripClear?()
                        self.onCharPickAlphabet?()
                        self.transition(to: .charPick)
                    }
                }
            }
        }
    }

    func switchLayer(to state: KeyboardState) {
        transition(to: state)
    }

    // MARK: - Candidate Panel Actions

    // Entry point for both the strip's chip tap and the panel's own chip row.
    // The strip needs the panel opened first; the panel's chip row is already
    // open, so this is a no-op there.
    func chipTapped(at index: Int) {
        guard let current = lastState else { return }
        let focused = current.focusedSegmentIndex.map { Int($0) } ?? 0
        let diff = index - focused
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            var state = current
            if diff > 0      { for _ in 0..<diff    { state = self.session.sendRight() } }
            else if diff < 0 { for _ in 0..<(-diff) { state = self.session.sendLeft()  } }
            self.dispatcher.onMain { [weak self] in
                guard let self else { return }
                if self.keyboardState != .panel { self.transition(to: .panel) }
                self.render(state)
            }
        }
    }

    func requestEdit(at index: Int) {
        guard let current = lastState else { return }
        let focused = current.focusedSegmentIndex.map { Int($0) } ?? 0
        let diff = index - focused
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            if diff > 0      { for _ in 0..<diff    { _ = self.session.sendRight() } }
            else if diff < 0 { for _ in 0..<(-diff) { _ = self.session.sendLeft()  } }
            let state = self.session.sendTab()
            self.dispatcher.onMain { [weak self] in
                guard let self else { return }
                self.render(state)
                self.transition(to: .qwerty)
            }
        }
    }

    func enterCharPickFromPanel() {
        for _ in romanBuffer { proxy.deleteBackward() }
        romanBuffer = ""
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            _ = self.session.enterCharPick()
            self.dispatcher.onMain { [weak self] in
                guard let self else { return }
                self.lastState = nil
                self.onStripClear?()
                self.onCharPickAlphabet?()
                self.transition(to: .charPick)
            }
        }
    }

    func charPickLetterTapped(_ letter: Character) {
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            let state = self.session.sendCharacter(String(letter))
            self.dispatcher.onMain { [weak self] in
                guard let self else { return }
                self.lastState = state
                self.onRender?(state, String(letter))
            }
        }
    }

    func selectCandidate(at index: Int) {
        if keyboardState == .charPick {
            if let candidate = lastState?.candidates[safe: index] {
                proxy.insertText(candidate)
            }
            dispatcher.onSession { [weak self] in
                guard let self else { return }
                _ = self.session.enterCharPick()
                self.dispatcher.onMain { [weak self] in
                    guard let self else { return }
                    self.lastState = nil
                    self.onStripClear?()
                    self.onCharPickAlphabet?()
                }
            }
            return
        }
        dispatcher.onSession { [weak self] in
            guard let self else { return }
            let state = self.session.selectCandidate(at: index)
            self.dispatcher.onMain { [weak self] in self?.render(state) }
        }
    }

    func dismissPanel() {
        if keyboardState == .charPick {
            dispatcher.onSession { [weak self] in
                guard let self else { return }
                _ = self.session.exitCharPick()
                self.dispatcher.onMain { [weak self] in self?.onStripClear?() }
            }
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
