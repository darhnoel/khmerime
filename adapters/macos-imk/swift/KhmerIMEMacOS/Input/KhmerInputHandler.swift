import Foundation

// Debounced model-refine scheduling. A protocol so tests can inject a synchronous fake;
// the production scheduler runs the block on the main queue after a pause. Mirrors the
// iOS DispatchModelRefineScheduler.
protocol MacosRefineTask: AnyObject {
    func cancel()
}

protocol MacosRefineScheduler {
    func schedule(after delay: TimeInterval, block: @escaping () -> Void) -> MacosRefineTask
}

final class DispatchMacosRefineScheduler: MacosRefineScheduler {
    func schedule(after delay: TimeInterval, block: @escaping () -> Void) -> MacosRefineTask {
        let task = DispatchMacosRefineTask(block: block)
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: task.workItem)
        return task
    }
}

private final class DispatchMacosRefineTask: MacosRefineTask {
    let workItem: DispatchWorkItem
    init(block: @escaping () -> Void) { workItem = DispatchWorkItem(block: block) }
    func cancel() { workItem.cancel() }
}

// Runs the model refine OFF the main thread, then delivers the result back ON the main thread for
// rendering. The model inference (refreshSegmentedPreview → refine_off_lock) can take up to ~1.3 s;
// running it inline on the main queue froze the keyboard UI for that whole time on every refine.
// The Rust side is already lock-free and generation-guarded, so it is safe to run off-thread.
// Mirrors the iOS QueuedDispatcher's onSession/onMain split. Tests inject a synchronous executor.
protocol MacosRefineExecutor {
    func run(work: @escaping () -> MacosRenderState, render: @escaping (MacosRenderState) -> Void)
}

final class DispatchMacosRefineExecutor: MacosRefineExecutor {
    private let queue = DispatchQueue(label: "com.khmerime.macos.refine", qos: .userInitiated)
    func run(work: @escaping () -> MacosRenderState, render: @escaping (MacosRenderState) -> Void) {
        queue.async {
            let state = work()
            DispatchQueue.main.async { render(state) }
        }
    }
}

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

    // MARK: - Model refine (debounced)

    private let refineScheduler: MacosRefineScheduler
    private let refineExecutor: MacosRefineExecutor
    private var pendingRefine: MacosRefineTask?
    // The pause after a keystroke before the model runs; matches iOS's 0.18 s.
    private static let refineDelay: TimeInterval = 0.18

    // MARK: - Init

    init(client: TextClient,
         session: MacosImkSession,
         refineScheduler: MacosRefineScheduler = DispatchMacosRefineScheduler(),
         refineExecutor: MacosRefineExecutor = DispatchMacosRefineExecutor()) {
        self.client = client
        self.session = session
        self.refineScheduler = refineScheduler
        self.refineExecutor = refineExecutor
    }

    /// After a keystroke with a live composition, run the model refine on a debounced pause (off the
    /// keystroke path). Uses refreshSegmentedPreview → refine_off_lock, the macOS segmented path
    /// that runs the model off-lock and merges its provenance-marked words into the candidate list.
    /// (refineComposition/apply_refined_candidate is a no-op here — it bails when a segmented session
    /// exists, which the macOS live path always builds.) No Smart gate: with no provider the session
    /// built no visible refiner, so it's a cheap no-op. The Rust generation guard drops results made
    /// stale by newer typing. Smart is implicit on macOS — on whenever a provider is armed (CONTEXT).
    private func scheduleModelRefine(for preedit: String) {
        pendingRefine?.cancel()
        guard !preedit.isEmpty else { return }
        pendingRefine = refineScheduler.schedule(after: Self.refineDelay) { [weak self] in
            guard let self else { return }
            // Run the model inference OFF the main thread (it can take up to ~1.3 s), then render its
            // result back ON the main thread. Running it inline here froze the keyboard for the whole
            // inference on every refine.
            self.refineExecutor.run(
                work: { self.session.refreshSegmentedPreview(rawPreedit: preedit) },
                render: { [weak self] state in self?.render(state) }
            )
        }
    }

    private func cancelPendingRefine() {
        pendingRefine?.cancel()
        pendingRefine = nil
    }

    // MARK: - Lifecycle

    @discardableResult
    func activate() -> MacosRenderState {
        session.activate()
    }

    func deactivate() {
        cancelPendingRefine()
        _ = session.deactivate()
        onPanelHide?()
    }

    /// Host app forces a commit (focus moves, mouse click in the document, …).
    func cancelComposition() {
        cancelPendingRefine()
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
        // Debounced model refine — only when the composition text actually changed. Navigation
        // and selection keys (Left/Right segment focus, Up/Down, digits, Tab) leave the preedit
        // unchanged; refining on those rebuilds the preview and resets the segment focus the user
        // just moved (arrows "always got refreshed" instead of editing). Gating on preeditChanged
        // keeps the model on real typing and lets focus/selection stand.
        if state.preeditChanged {
            scheduleModelRefine(for: state.preedit)
        } else {
            // The composition text didn't change — this was navigation/selection (Space cycle,
            // arrows, digits). Kill any refine still pending from the last typed letter, so it
            // can't fire mid-selection and rebuild the candidate list back to the top word.
            cancelPendingRefine()
        }
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
            // The composition is gone; drop any pending refine so it can't fire post-commit.
            cancelPendingRefine()
        }
        // Mirror the preedit, and clear it exactly once when the composition ends.
        // Both halves matter: skipping the empty call strands the last character in
        // the client, while sending it on every render — including keys that never
        // composed — spams clients that track marked state and wedges the picky ones
        // (Notes stops accepting input). `preeditChanged` (stamped in Rust by comparing
        // against the previous preedit) gates it to the transitions that matter: write
        // while composing, and write the empty string only on the marked → unmarked edge.
        if !state.preedit.isEmpty {
            client.setMarkedText(state.preedit)
            hasMarkedText = true
        } else if hasMarkedText && state.preeditChanged {
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
