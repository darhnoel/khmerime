import Cocoa
import InputMethodKit
import os

// KhmerInputController
// ====================
// IMKInputController subclass — one instance per text client. THIN SHELL:
// all logic lives in KhmerInputHandler (pure Swift, unit-tested); this class
// only adapts IMK types at the boundary and positions the panel.
//
//   IMK callback            →  handler call
//   ─────────────────────────────────────────────
//   activateServer:         →  activate()
//   deactivateServer:       →  deactivate()
//   commitComposition:      →  cancelComposition()
//   handle(_:client:)       →  handleKey(keyval:macKeycode:modifierFlags:)
//
// Render loop invariant (same as all other adapters): every key event goes
// through the session; MacosRenderState is the single source of truth.

@objc(KhmerInputController)
class KhmerInputController: IMKInputController {

    private var handler: KhmerInputHandler!
    private let panel = CandidatePanel()

    override init!(server: IMKServer!, delegate: Any!, client inputClient: Any!) {
        super.init(server: server, delegate: delegate, client: inputClient)
        let imkClient = IMKTextClientAdapter(
            client: { [weak self] in self?.client() },
            markAttributes: { [weak self] in
                self?.mark(
                    forStyle: kTSMHiliteSelectedConvertedText,
                    at: NSRange(location: NSNotFound, length: NSNotFound)
                ) as? [NSAttributedString.Key: Any] ?? [:]
            }
        )
        handler = KhmerInputHandler(client: imkClient, session: MacosImkSession())
        handler.onPanelUpdate = { [weak self] state in
            guard let self else { return }
            self.panel.update(state)
            if let input = self.client() {
                // Anchor on the last marked glyph's line-height rectangle — a true
                // line rect (proper height + baseline), unlike firstRect's
                // top-anchored marked-text rect. The panel follows the caret as the
                // composition grows.
                let index = CandidatePanelLayout.caretAnchorIndex(preedit: state.preedit)
                var lineRect = NSRect.zero
                _ = input.attributes(forCharacterIndex: index, lineHeightRectangle: &lineRect)
                self.logCaretRect(index: index, lineRect: lineRect)
                self.panel.show(below: lineRect)
            }
        }
        handler.onPanelHide = { [weak self] in
            self?.panel.hide()
        }
    }

    // MARK: - Lifecycle

    override func activateServer(_ sender: Any!) {
        handler.activate()
    }

    override func deactivateServer(_ sender: Any!) {
        handler.deactivate()
    }

    override func commitComposition(_ sender: Any!) {
        handler.cancelComposition()
    }

    // MARK: - Key handling

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown else { return false }
        let input = sessionKeyInput(from: event)
        return handler.handleKey(
            keyval: input.keyval,
            macKeycode: input.keycode,
            modifierFlags: input.modifierFlags
        )
    }

    // MARK: - TEMP ground-truth capture (remove with the positioning refactor)

    // Logs the line rect attributes(forCharacterIndex:) returns, at .notice level
    // so `log show` can retrieve it (unlike .debug, which is not persisted).
    // See docs/refactor-candidate-panel-positioning.md.
    private func logCaretRect(index: Int, lineRect: NSRect) {
        Logger(subsystem: "com.khmerime.inputmethod.KhmerIMEMacOS", category: "anchor")
            .notice("[DEBUG-macos-imk-runtime] index=\(index) lineRect=\(NSStringFromRect(lineRect), privacy: .public)")
    }

    // MARK: - Cursor tracking

    func updateCursorRect(_ rect: NSRect) {
        handler.setCursorRect(
            x: Int32(rect.origin.x),
            y: Int32(rect.origin.y),
            width: Int32(rect.size.width),
            height: Int32(rect.size.height)
        )
    }

    // MARK: - Input mode toggle

    func toggleInputMode() {
        handler.toggleInputMode()
    }
}

// MARK: - IMKTextClientAdapter

// Bridges TextClient to the live IMKTextInput object. The client is resolved
// per call (IMK can hand the controller a different sender over time), and
// marked-text attributes come from IMKInputController.mark(forStyle:at:).
private final class IMKTextClientAdapter: TextClient {

    private let client: () -> IMKTextInput?
    private let markAttributes: () -> [NSAttributedString.Key: Any]

    init(client: @escaping () -> IMKTextInput?,
         markAttributes: @escaping () -> [NSAttributedString.Key: Any]) {
        self.client = client
        self.markAttributes = markAttributes
    }

    func insertText(_ text: String) {
        client()?.insertText(
            text,
            replacementRange: NSRange(location: NSNotFound, length: NSNotFound)
        )
    }

    func setMarkedText(_ text: String) {
        guard let input = client() else { return }
        let attrStr = NSAttributedString(string: text, attributes: markAttributes())
        input.setMarkedText(
            attrStr,
            selectionRange: NSRange(location: text.utf16.count, length: 0),
            replacementRange: NSRange(location: NSNotFound, length: NSNotFound)
        )
    }
}
