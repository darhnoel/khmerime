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

    // MARK: - Debounced visible refine
    //
    // Live keystrokes stay on the cheap deterministic engine. ~200ms after the user
    // pauses, we run the heavier visible refiner (incl. the byT5 model) once and update
    // the panel — keeping typing instant while still surfacing the model's candidates.
    private var refineTimer: Timer?
    private let refineDelay: TimeInterval = 0.2

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
        refineTimer?.invalidate()
        refineTimer = nil
        _ = session.deactivate()
        onPanelHide?()
    }

    /// Host app forces a commit (focus moves, mouse click in the document, …).
    func cancelComposition() {
        refineTimer?.invalidate()
        refineTimer = nil
        render(session.cancelComposition())
    }

    // MARK: - Key handling

    func handleKey(keyval: UInt32, macKeycode: UInt16, modifierFlags: UInt32) -> Bool {
        let state = session.handleEvent(keyval: keyval, macKeycode: macKeycode, modifierFlags: modifierFlags)
        render(state)
        // Only (re)arm the pause-refine when the key actually extends the composition
        // (a roman letter). Navigation/selection keys (arrows, digits, space, enter, tab…)
        // CANCEL any pending refine so it can never fire mid-selection and reset focus.
        if isComposingLetter(keyval) {
            scheduleVisibleRefine(state)
        } else {
            refineTimer?.invalidate()
            refineTimer = nil
        }
        return state.consumed
    }

    /// True for ASCII a–z / A–Z, the only keys that extend a roman composition.
    private func isComposingLetter(_ keyval: UInt32) -> Bool {
        (keyval >= 0x61 && keyval <= 0x7A) || (keyval >= 0x41 && keyval <= 0x5A)
    }

    /// Arm the debounce: ~200ms after the last typed letter (no further keys), run the
    /// visible refiner once. Only active while a composition is showing.
    private func scheduleVisibleRefine(_ state: MacosRenderState) {
        refineTimer?.invalidate()
        refineTimer = nil
        guard !state.preedit.isEmpty else { return }
        refineTimer = Timer.scheduledTimer(withTimeInterval: refineDelay, repeats: false) { [weak self] _ in
            guard let self = self else { return }
            self.render(self.session.refineVisible())
        }
    }

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
        if !state.preedit.isEmpty || state.commitText != nil {
            client.setMarkedText(state.preedit)
        }
        if state.candidates.isEmpty && state.segments.isEmpty {
            onPanelHide?()
        } else {
            onPanelUpdate?(state)
        }
    }
}
