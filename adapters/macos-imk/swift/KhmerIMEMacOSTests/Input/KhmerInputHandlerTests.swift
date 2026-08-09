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

    // A refine scheduler that captures the pending block (and its cancellation) instead of
    // running it on a timer, so a test can fire — or prove the cancellation of — a debounced
    // refine deterministically.
    private final class ManualRefineScheduler: MacosRefineScheduler {
        final class Task: MacosRefineTask {
            var block: (() -> Void)?
            var cancelled = false
            func cancel() { cancelled = true; block = nil }
        }
        private(set) var latest: Task?
        func schedule(after delay: TimeInterval, block: @escaping () -> Void) -> MacosRefineTask {
            let task = Task()
            task.block = block
            latest = task
            return task
        }
        /// Fire the most recently scheduled block if it wasn't cancelled.
        func fireLatest() { latest?.block?() }
    }

    private func makeHandler(scheduler: MacosRefineScheduler) -> (KhmerInputHandler, MockTextClient) {
        let client = MockTextClient()
        let handler = KhmerInputHandler(client: client, session: MacosImkSession(), refineScheduler: scheduler)
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

    // Backspacing the whole Composition away must clear the host's marked range.
    // An empty marked string is the only way to tell the client "there is no
    // preedit"; skipping the call leaves the last character stuck on screen.
    func test_backspacingWholeCompositionClearsMarkedText() {
        let (handler, client) = makeHandler()
        type("nhom", into: handler)
        XCTAssertEqual(client.markedText, "nhom", "precondition: preedit is showing")

        for _ in 0..<4 {
            _ = handler.handleKey(keyval: 0xFF08, macKeycode: 0, modifierFlags: 0) // BackSpace
        }

        XCTAssertEqual(client.markedText, "",
            "the last Backspace must clear the marked text, not leave it stuck")
        XCTAssertTrue(client.text.isEmpty,
            "backspacing a composition away must not commit anything")
    }

    // Clearing the marked range is only correct when there was something marked.
    // Sending setMarkedText("") on every render — including keys that never composed —
    // wedges picky IMK clients (Notes stops accepting input entirely).
    func test_keysThatNeverCompose_doNotSpamEmptyMarkedText() {
        let (handler, client) = makeHandler()

        // A ⌘-combo passes through without composing; it must not touch marked text.
        _ = handler.handleKey(
            keyval: 0x63, macKeycode: 8,
            modifierFlags: UInt32(NSEvent.ModifierFlags.command.rawValue)
        )

        XCTAssertTrue(client.ops.isEmpty,
            "a key that never composed must not send any marked-text op; got \(client.ops)")
    }

    // Composition-Consuming Enter (ADR-0017): with no Composition active, Enter must
    // NOT be consumed so the host application decides what Return means — send the
    // message, insert a newline, trigger the default button. Consuming an idle Enter
    // is what made a committing Enter also send on Facebook.
    func test_enterWithoutComposition_isNotConsumed() {
        let (handler, _) = makeHandler()

        let consumed = handler.handleKey(keyval: 0xFF0D, macKeycode: 0, modifierFlags: 0)

        XCTAssertFalse(consumed,
            "an idle Enter must pass through to the application (ADR-0017)")
    }

    func test_enterWhileComposing_isConsumed() {
        let (handler, _) = makeHandler()
        type("nhom", into: handler)

        let consumed = handler.handleKey(keyval: 0xFF0D, macKeycode: 0, modifierFlags: 0)

        XCTAssertTrue(consumed,
            "the Enter that commits a Composition must be swallowed, so the host never sees a Return")
    }

    // Command-modified keys are never text input on macOS: ⌘A / ⌘C / ⌘V belong to the
    // application. Composing them as Khmer (the Command bit was not mapped, so ⌘C
    // arrived as a bare 'c') swallowed the user's copy/paste shortcuts.
    func test_commandModifiedKey_isNotConsumedAndTypesNothing() {
        let (handler, client) = makeHandler()
        let command = UInt32(NSEvent.ModifierFlags.command.rawValue)

        let consumed = handler.handleKey(keyval: 0x63, macKeycode: 8, modifierFlags: command) // ⌘C

        XCTAssertFalse(consumed,
            "⌘-combos must pass through to the application, not be consumed as input")
        XCTAssertTrue(client.text.isEmpty && client.markedText.isEmpty,
            "⌘C must not compose or commit any text")
    }

    func test_spaceSelectsNextCandidateAndEnterCommitsVisibleChoice() {
        let (handler, client) = makeHandler()
        var updated: MacosRenderState?
        handler.onPanelUpdate = { updated = $0 }
        type("jea", into: handler)
        let before = updated

        let consumedSpace = handler.handleKey(keyval: 0x20, macKeycode: 0, modifierFlags: 0)
        let afterSpace = updated

        XCTAssertTrue(consumedSpace, "Space must be consumed while candidate UI is active")
        XCTAssertEqual(client.text, "", "Space must select/cycle, not commit or insert a literal space")
        XCTAssertEqual(client.markedText, "jea", "raw roman preedit must remain marked while selecting")
        XCTAssertEqual(before?.selectedIndex, 0)
        XCTAssertEqual(afterSpace?.selectedIndex, 1)
        guard let selectedIndex = afterSpace?.selectedIndex,
              let selected = afterSpace?.candidates[Int(selectedIndex)] else {
            return XCTFail("Space must leave a visible selected candidate")
        }

        _ = handler.handleKey(keyval: 0xFF0D, macKeycode: 0, modifierFlags: 0)

        XCTAssertEqual(client.text, selected,
            "Enter must commit the visible candidate selected by Space")
    }

    func test_spaceCyclingCancelsPendingRefineSoSelectionSurvives() {
        // Regression ("space space space then refresh to the top word"): typing schedules a
        // debounced refine; pressing Space to cycle must CANCEL that pending refine, otherwise
        // it fires mid-selection, rebuilds the candidate list, and snaps selection back to 0.
        let scheduler = ManualRefineScheduler()
        let (handler, _) = makeHandler(scheduler: scheduler)
        var updated: MacosRenderState?
        handler.onPanelUpdate = { updated = $0 }

        type("jea", into: handler) // last letter schedules a refine (still pending)
        XCTAssertNotNil(scheduler.latest, "typing must schedule a debounced refine")

        // Cycle three times with Space.
        for _ in 0..<3 { _ = handler.handleKey(keyval: 0x20, macKeycode: 0, modifierFlags: 0) }
        XCTAssertEqual(updated?.selectedIndex, 3, "three Spaces select candidate 3")
        XCTAssertEqual(scheduler.latest?.cancelled, true, "Space must cancel the pending refine")

        // Even if that stale refine somehow fires, it's a no-op (cancelled → block cleared).
        scheduler.fireLatest()
        XCTAssertEqual(updated?.selectedIndex, 3, "selection must survive; no reset to the top word")
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

    // MARK: - Backspace / marked text lifecycle

    func test_backspaceLastChar_clearsMarkedText() {
        let (handler, client) = makeHandler()
        type("k", into: handler)
        XCTAssertEqual(client.markedText, "k")

        _ = handler.handleKey(keyval: 0xFF08, macKeycode: 0, modifierFlags: 0) // Backspace

        XCTAssertEqual(client.markedText, "",
            "backspace on last char must clear marked text — preeditChanged must fire on empty transition")
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
