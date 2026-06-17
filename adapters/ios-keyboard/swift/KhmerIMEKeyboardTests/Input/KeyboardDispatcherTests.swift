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

    func test_sendChar_deferredRenderDoesNotFireBeforeOnMain() {
        // sendChar fires one optimistic render immediately (roman-hint update),
        // then a deferred render inside the onMain block (Rust-confirmed candidates).
        // This test verifies the deferred render stays deferred until onMain runs.
        let dispatcher = CapturingDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()

        var renderCount = 0
        handler.onRender = { _, _ in renderCount += 1 }

        handler.sendChar("k")
        XCTAssertEqual(renderCount, 1, "optimistic render must fire immediately")

        dispatcher.capturedSession?()   // runs session block, queues onMain
        XCTAssertEqual(renderCount, 1, "deferred render must not fire before onMain runs")

        dispatcher.capturedMain?()      // runs the main callback
        XCTAssertEqual(renderCount, 2, "deferred render must fire inside the onMain callback")
    }

    // MARK: - Optimistic render

    // When lastState exists (i.e. at least one prior keystroke has been processed),
    // sendChar must fire onRender immediately — before the session block executes —
    // so the strip roman-hint row updates without waiting for Rust.
    func test_backspaceTapped_withNonEmptyBuffer_firesOptimisticRenderImmediately() {
        let dispatcher = CapturingDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()
        handler.sendChar("k"); handler.sendChar("h")   // romanBuffer = "kh", lastState set

        var renderCount = 0
        var firstRenderHint: String?
        handler.onRender = { _, hint in
            renderCount += 1
            if renderCount == 1 { firstRenderHint = hint }
        }

        handler.backspaceTapped()   // romanBuffer becomes "k" (non-empty)
        // Session block captured but NOT yet executed.
        XCTAssertEqual(renderCount, 1,
            "backspaceTapped must fire onRender optimistically when buffer stays non-empty")
        XCTAssertEqual(firstRenderHint, "k",
            "optimistic render must use the updated romanBuffer after the deletion")
    }

    func test_sendChar_withExistingLastState_firesOptimisticRenderImmediately() {
        let dispatcher = CapturingDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()   // sets lastState

        var renderCount = 0
        handler.onRender = { _, _ in renderCount += 1 }

        handler.sendChar("k")
        // Session block captured but NOT yet executed.
        XCTAssertEqual(renderCount, 1,
            "sendChar must fire onRender optimistically before the session block runs")
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

        // Each char fires twice: optimistic (stale previous preedit) then Rust-confirmed.
        // focusIn() produces an empty-preedit lastState, so the first optimistic render is "".
        XCTAssertEqual(preeditHistory, ["", "k", "k", "kh", "kh", "khn"],
            "renders must arrive in keystroke order — optimistic (stale preedit) then Rust-confirmed per char")
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
