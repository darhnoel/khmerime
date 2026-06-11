import XCTest

// KhmerInputHandlerTests
// ======================
// Integration-style tests: real Rust session, mock text client.
// Mirrors KeyboardInputHandlerTests on iOS — behavior through the public
// interface, never internal state.

final class KhmerInputHandlerTests: XCTestCase {

    // Waits for the background warmup (same retry loop as the Rust
    // protocol tests — activate() reports isReady).
    private func makeHandler() -> (KhmerInputHandler, MockTextClient) {
        let client = MockTextClient()
        let handler = KhmerInputHandler(client: client, session: MacosImkSession())
        for _ in 0..<100 {
            if handler.activate().isReady { break }
            Thread.sleep(forTimeInterval: 0.1)
        }
        return (handler, client)
    }

    private func type(_ word: String, into handler: KhmerInputHandler) {
        for ch in word.unicodeScalars {
            _ = handler.handleKey(keyval: ch.value, macKeycode: 0, modifierFlags: 0)
        }
    }

    // MARK: - Tracer bullet

    func test_typeAndEnter_commitsKhmerToClient() {
        let (handler, client) = makeHandler()
        type("nhom", into: handler)

        _ = handler.handleKey(keyval: 0xFF0D, macKeycode: 0, modifierFlags: 0) // Enter

        XCTAssertFalse(client.text.isEmpty,
            "Enter must commit text into the client")
        XCTAssertFalse(client.text.contains("nhom"),
            "committed text must be Khmer, not the roman preedit")
        let isKhmer = client.text.unicodeScalars.allSatisfy { (0x1780...0x17FF).contains($0.value) }
        XCTAssertTrue(isKhmer,
            "committed text must be Khmer Unicode only; got \(client.text.debugDescription)")
    }

    // MARK: - Render ordering

    func test_typing_updatesMarkedTextWithRomanPreedit() {
        let (handler, client) = makeHandler()

        type("nh", into: handler)

        XCTAssertEqual(client.markedText, "nh",
            "marked text must mirror the raw roman preedit (ADR-0003)")
        XCTAssertTrue(client.text.isEmpty,
            "nothing may be committed while composing")
    }

    func test_commit_insertsTextBeforeClearingMarkedText() {
        // IMK contract: insertText must reach the client before the marked
        // text is cleared, otherwise the host briefly shows neither.
        let (handler, client) = makeHandler()
        type("nhom", into: handler)

        _ = handler.handleKey(keyval: 0xFF0D, macKeycode: 0, modifierFlags: 0)

        guard let insertIdx = client.ops.firstIndex(where: {
            if case .insert = $0 { return true } else { return false }
        }) else {
            return XCTFail("no insert op recorded")
        }
        let after = client.ops[client.ops.index(after: insertIdx)...]
        XCTAssertTrue(after.contains(.marked("")),
            "marked text must be cleared AFTER the commit insert; ops: \(client.ops)")
    }

    // MARK: - Panel callbacks (display-only panel: handler decides show/hide)

    func test_typing_firesPanelUpdateWithCandidates() {
        let (handler, _) = makeHandler()
        var updated: MacosRenderState?
        handler.onPanelUpdate = { updated = $0 }

        type("nhom", into: handler)

        XCTAssertNotNil(updated, "typing must fire onPanelUpdate")
        XCTAssertFalse(updated?.candidates.isEmpty ?? true,
            "panel update must carry candidates for the composition")
    }

    func test_commit_firesPanelHide() {
        let (handler, _) = makeHandler()
        var hidden = false
        handler.onPanelHide = { hidden = true }
        type("nhom", into: handler)

        _ = handler.handleKey(keyval: 0xFF0D, macKeycode: 0, modifierFlags: 0)

        XCTAssertTrue(hidden,
            "panel must hide after commit empties candidates and segments")
    }

    func test_deactivate_firesPanelHide() {
        let (handler, _) = makeHandler()
        var hidden = false
        handler.onPanelHide = { hidden = true }
        type("nh", into: handler)

        handler.deactivate()

        XCTAssertTrue(hidden, "losing focus must hide the panel")
    }
}
