import Cocoa
import InputMethodKit

// KhmerInputController
// ====================
// IMKInputController subclass — one instance per text client.
//
// Session lifecycle (mirrors KeyboardSession.swift on iOS):
//   activateServer:   → session.activate()      (focus acquired)
//   deactivateServer: → session.deactivate()     (focus lost)
//   commitComposition → session.cancelComposition() (host forces commit)
//   handle(_:client:) → session.handleEvent(…)  (every key press)
//
// Render loop invariant (same as all other adapters):
//   Every key event goes through the session. Never insert text without
//   consulting the session first. MacosRenderState is the single source
//   of truth for preedit, candidates, segments, and commit text.
//
// Marked text (preedit) contract (ADR-0003):
//   state.preedit is the raw roman string (e.g. "khnhomtov").
//   Swift passes this to setMarkedText: so the host app underlines it.
//   Khmer is displayed only in the CandidatePanel (chips + candidates row).

@objc(KhmerInputController)
class KhmerInputController: IMKInputController {

    private let session: MacosImkSession
    private lazy var panel = CandidatePanel(controller: self)

    override init!(server: IMKServer!, delegate: Any!, client inputClient: Any!) {
        session = MacosImkSession()
        super.init(server: server, delegate: delegate, client: inputClient)
    }

    // MARK: - Lifecycle

    override func activateServer(_ sender: Any!) {
        let state = session.activate()
        render(state, to: sender)
    }

    override func deactivateServer(_ sender: Any!) {
        session.deactivate()
        panel.hide()
    }

    override func commitComposition(_ sender: Any!) {
        let state = session.cancelComposition()
        applyCommit(state, to: sender)
        panel.hide()
    }

    // MARK: - Key handling

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown else { return false }

        let keyval  = keyvalFromNSEvent(event)
        let macKc   = UInt16(event.keyCode)
        let mods    = UInt32(event.modifierFlags.rawValue)
        let state   = session.handleEvent(keyval: keyval, macKeycode: macKc, modifierFlags: mods)

        render(state, to: sender)
        return state.consumed
    }

    // MARK: - Cursor tracking

    override func didCommand(by aSelector: Selector!, client sender: Any!) -> Bool {
        return false
    }

    // Called by the IMK framework when the insertion point rect changes.
    func updateCursorRect(_ rect: NSRect) {
        let state = session.setCursorLocation(
            x: Int32(rect.origin.x),
            y: Int32(rect.origin.y),
            width: Int32(rect.size.width),
            height: Int32(rect.size.height)
        )
        if !state.candidates.isEmpty || !state.segments.isEmpty {
            panel.show(below: rect)
        }
    }

    // MARK: - Input mode toggle (bound to keyboard shortcut in CandidatePanel)

    func toggleInputMode() {
        let state = session.toggleInputMode()
        render(state, to: client())
    }

    // MARK: - Candidate panel callbacks

    func panelDidSelectCandidate(at index: Int) {
        guard index >= 0 else { return }
        let keyval = UInt32(0x31) + UInt32(index) // '1'…'9'
        let state  = session.handleEvent(keyval: keyval, macKeycode: 0, modifierFlags: 0)
        render(state, to: client())
    }

    func panelDidTapChip(at index: Int) {
        // Right-arrow to that segment: send index - focused_segment_index rights
        let state = session.handleEvent(keyval: 0xFF53, macKeycode: 0, modifierFlags: 0)
        render(state, to: client())
    }

    func panelDidRequestEdit(at index: Int) {
        let state = session.handleEvent(keyval: 0xFF09, macKeycode: 0, modifierFlags: 0) // Tab
        render(state, to: client())
    }

    // MARK: - Render

    private func render(_ state: MacosRenderState, to client: Any?) {
        applyCommit(state, to: client)
        applyPreedit(state, to: client)
        updatePanel(state, client: client)
    }

    private func applyCommit(_ state: MacosRenderState, to client: Any?) {
        guard let text = state.commitText, !text.isEmpty else { return }
        (client as? IMKTextInput)?.insertText(
            text,
            replacementRange: NSRange(location: NSNotFound, length: NSNotFound)
        )
    }

    private func applyPreedit(_ state: MacosRenderState, to client: Any?) {
        guard let input = client as? IMKTextInput else { return }
        if state.preedit.isEmpty && state.commitText == nil { return }
        let attrs = mark(forStyle: kTSMHiliteSelectedConvertedText, at: NSRange(location: NSNotFound, length: NSNotFound))
        let attrStr = NSAttributedString(
            string: state.preedit,
            attributes: attrs as? [NSAttributedString.Key: Any] ?? [:]
        )
        let sel = NSRange(location: state.preedit.utf16.count, length: 0)
        input.setMarkedText(
            attrStr,
            selectionRange: sel,
            replacementRange: NSRange(location: NSNotFound, length: NSNotFound)
        )
    }

    private func updatePanel(_ state: MacosRenderState, client: Any?) {
        if state.candidates.isEmpty && state.segments.isEmpty {
            panel.hide()
        } else {
            panel.update(state)
            if let input = client as? IMKTextInput {
                let rect = input.firstRect(
                    forCharacterRange: NSRange(location: NSNotFound, length: NSNotFound),
                    actualRange: nil
                )
                panel.show(below: rect)
            }
        }
    }
}

// MARK: - Keyval mapping

/// Convert an NSEvent to an XKB keyval.
///
/// For printable ASCII keys the Unicode scalar is the keyval.
/// For special keys (arrows, Return, Backspace, Tab, Escape) we map to
/// X11 keysym constants matching what the Linux IBus adapter delivers —
/// the Rust session uses these same constants (KEY_RETURN = 0xFF0D, etc.).
private func keyvalFromNSEvent(_ event: NSEvent) -> UInt32 {
    switch Int(event.keyCode) {
    case kVK_Return, kVK_ANSI_KeypadEnter: return 0xFF0D
    case kVK_Delete:                        return 0xFF08
    case kVK_Tab:                           return 0xFF09
    case kVK_Escape:                        return 0xFF1B
    case kVK_Space:                         return 0x0020
    case kVK_LeftArrow:                     return 0xFF51
    case kVK_RightArrow:                    return 0xFF53
    case kVK_UpArrow:                       return 0xFF52
    case kVK_DownArrow:                     return 0xFF54
    default:
        // For printable keys, use the Unicode scalar of the unmodified character.
        // characters(ignoringModifiers:) strips Shift so 'A' → 65 and 'a' → 97
        // but NIDA mode re-applies the Shift from modifierFlags via xkb_state.
        if let ch = event.characters(byApplyingModifiers: [])?.unicodeScalars.first {
            return ch.value
        }
        return event.characters?.unicodeScalars.first?.value ?? 0
    }
}
