import Foundation

// KhmerInputHandler
// =================
// Pure-Swift input logic extracted from KhmerInputController so it can be
// unit-tested without InputMethodKit. Mirrors the IBus engine shape: a thin
// forwarder — every key event goes to the Rust session, and MacosRenderState
// is the single source of truth for preedit, candidates, segments, commit.
//
// The handler owns the render loop:
//   1. commit text   → client.insertText      (must happen first)
//   2. preedit       → client.setMarkedText
//   3. panel content → onPanelUpdate / onPanelHide callbacks
//
// All candidate/segment interaction is keyboard-driven (arrows, Space, digits,
// Tab, Enter) and interpreted by the session — the panel is display-only.

final class KhmerInputHandler {

    let client: TextClient
    let session: MacosImkSession

    // MARK: - UI Callbacks

    /// Panel has content to show; the controller positions it at the cursor.
    var onPanelUpdate: ((MacosRenderState) -> Void)?
    /// Panel must hide (no candidates, no segments).
    var onPanelHide: (() -> Void)?

    /// Whether the client currently holds a marked range, so the empty-string clear is
    /// sent once on the marked → unmarked transition rather than on every render.
    private var hasMarkedText = false

    // MARK: - Init

    init(client: TextClient, session: MacosImkSession) {
        self.client = client
        self.session = session
    }

    // MARK: - Lifecycle

    @discardableResult
    func activate() -> MacosRenderState {
        session.activate()
    }

    func deactivate() {
        _ = session.deactivate()
        onPanelHide?()
    }

    /// Host app forces a commit (focus moves, mouse click in the document, …).
    func cancelComposition() {
        render(session.cancelComposition())
    }

    // MARK: - Key handling

    func handleKey(keyval: UInt32, macKeycode: UInt16, modifierFlags: UInt32) -> Bool {
        // ⌘-combos (⌘A / ⌘C / ⌘V / ⌘Z …) are application commands, never text input.
        // The session has no notion of Command — without this guard ⌘C arrives as a
        // bare 'c', composes as Khmer, and reports itself consumed, swallowing the
        // shortcut. Finish any live Composition first so the commit is not lost, then
        // decline the event so macOS routes it to the application.
        if modifierFlags & Self.commandKeyMask != 0 {
            render(session.cancelComposition())
            return false
        }

        let state = session.handleEvent(keyval: keyval, macKeycode: macKeycode, modifierFlags: modifierFlags)
        render(state)
        return state.consumed
    }

    /// `NSEvent.ModifierFlags.command` — the raw AppKit bit, kept here so the guard
    /// works on the primitive the controller passes across the boundary.
    private static let commandKeyMask: UInt32 = 1 << 20

    // MARK: - Cursor / mode

    func setCursorRect(x: Int32, y: Int32, width: Int32, height: Int32) {
        render(session.setCursorLocation(x: x, y: y, width: width, height: height))
    }

    func toggleInputMode() {
        render(session.toggleInputMode())
    }

    // MARK: - Render loop

    private func render(_ state: MacosRenderState) {
        if let text = state.commitText, !text.isEmpty {
            client.insertText(text)
        }
        // Mirror the preedit, and clear it exactly once when the composition ends.
        // Both halves matter: skipping the empty call strands the last character in
        // the client, while sending it on every render — including keys that never
        // composed — spams clients that track marked state and wedges the picky ones
        // (Notes stops accepting input). So: write while marking, and write the empty
        // string only on the marked → unmarked transition.
        if !state.preedit.isEmpty {
            client.setMarkedText(state.preedit)
            hasMarkedText = true
        } else if hasMarkedText {
            client.setMarkedText("")
            hasMarkedText = false
        }
        if state.candidates.isEmpty && state.segments.isEmpty {
            onPanelHide?()
        } else {
            onPanelUpdate?(state)
        }
    }
}
