import Foundation

// KeyboardDispatcher
// ==================
// Separates "where session work runs" from the input logic in
// KeyboardInputHandler. Two concrete implementations:
//
//   QueuedDispatcher   — production: session on a serial background queue,
//                        main callbacks on DispatchQueue.main. Frees the
//                        main thread from Rust segmentation latency per key.
//
//   SynchronousDispatcher — tests: both blocks execute inline on the calling
//                           thread. No XCTestExpectation needed; all existing
//                           tests remain synchronous.
//
// For operations where proxy mutations depend on the session result
// (space/return/commit), the handler dispatches the proxy write inside
// the onMain block so ordering is always: delete-roman → session → insert-Khmer.
// For letter keys the proxy write is immediate (before onSession), because
// letters never produce commitText and the speculative roman char must appear
// in the text field without any delay.

protocol KeyboardDispatcher: AnyObject {
    // Runs `work` on the session processing queue (serial, background in production).
    func onSession(_ work: @escaping () -> Void)

    // Runs `work` on the main thread (or inline in tests).
    // Always called from within an `onSession` block.
    func onMain(_ work: @escaping () -> Void)
}

// MARK: - Production

final class QueuedDispatcher: KeyboardDispatcher {
    private let queue: DispatchQueue

    init(label: String = "com.khmerime.session", qos: DispatchQoS = .userInteractive) {
        queue = DispatchQueue(label: label, qos: qos)
    }

    func onSession(_ work: @escaping () -> Void) {
        queue.async { work() }
    }

    func onMain(_ work: @escaping () -> Void) {
        DispatchQueue.main.async { work() }
    }
}

// MARK: - Tests

final class SynchronousDispatcher: KeyboardDispatcher {
    func onSession(_ work: @escaping () -> Void) { work() }
    func onMain(_ work: @escaping () -> Void)    { work() }
}
