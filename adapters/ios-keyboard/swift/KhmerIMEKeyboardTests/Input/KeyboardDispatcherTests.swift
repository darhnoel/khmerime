import XCTest
@testable import KhmerIMEKeyboard

// KeyboardDispatcherTests
// =======================
// Verifies the dispatcher contract through three behaviors:
//   1. proxy.insertText is immediate — fires before the session block executes
//   2. render fires inside onMain, not before it
//   3. multiple chars are processed in declaration order (session serial contract)

final class KeyboardDispatcherTests: XCTestCase {

    // MARK: - Proxy insertion is immediate

    // Uses CapturingDispatcher to freeze session work mid-flight and confirm
    // the text field already contains the typed char before session runs.
    func test_sendChar_insertsIntoProxyBeforeSessionFires() {
        let dispatcher = CapturingDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()

        handler.sendChar("k")

        // Session block was captured but NOT yet executed.
        XCTAssertNotNil(dispatcher.capturedSession, "session work must be dispatched")
        XCTAssertEqual(proxy.text, "k",
            "proxy must contain 'k' immediately — insertion must not wait for the session block")
    }

    // MARK: - Render fires inside onMain

    func test_sendChar_renderDoesNotFireBeforeSessionWork() {
        let dispatcher = CapturingDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()

        var renderFired = false
        handler.onRender = { _, _ in renderFired = true }

        handler.sendChar("k")
        XCTAssertFalse(renderFired, "render must not fire before the session block runs")
    }

    func test_sendChar_renderFiresAfterSessionThenMain() {
        let dispatcher = CapturingDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()

        var renderFired = false
        handler.onRender = { _, _ in renderFired = true }

        handler.sendChar("k")
        dispatcher.capturedSession?()   // runs session work, which dispatches to onMain
        XCTAssertFalse(renderFired, "render must not fire before onMain runs")

        dispatcher.capturedMain?()      // runs the main callback
        XCTAssertTrue(renderFired, "render must fire inside the onMain callback")
    }

    // MARK: - Multiple chars processed in order

    func test_sendChar_multipleChars_rendersArriveInKeystrokeOrder() {
        let dispatcher = SynchronousDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()

        var preeditHistory: [String] = []
        handler.onRender = { state, _ in preeditHistory.append(state.preedit) }

        handler.sendChar("k")
        handler.sendChar("h")
        handler.sendChar("n")

        XCTAssertEqual(preeditHistory, ["k", "kh", "khn"],
            "renders must arrive in keystroke order — session is serial")
    }
}

// MARK: - Test Doubles

/// Captures session and main blocks without executing them.
/// Lets tests verify what-fires-when without concurrency.
final class CapturingDispatcher: KeyboardDispatcher {
    var capturedSession: (() -> Void)?
    var capturedMain: (() -> Void)?

    func onSession(_ work: @escaping () -> Void) {
        capturedSession = work
    }

    func onMain(_ work: @escaping () -> Void) {
        capturedMain = work
    }
}
