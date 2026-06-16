import UIKit

// BackspaceButton
// ===============
// GlassKeyButton subclass that adds long-press repeat — the standard iOS
// keyboard feel where holding ⌫ fires one delete per 100 ms after a 400 ms
// initial delay. The repeater logic is isolated in BackspaceRepeater so it
// can be tested without any UIKit dependency.
//
// Wiring (in KeyboardViewController):
//   btn.onDelete = { [weak self] in self?.handler.backspaceTapped() }
//
// touchesEnded falls through to a single-tap delete only when the repeater
// has NOT already fired (i.e. the user tapped rather than held).

// MARK: - Scheduler protocol

protocol BackspaceScheduler {
    func schedule(after delay: TimeInterval, block: @escaping () -> Void) -> BackspaceTask
}

protocol BackspaceTask: AnyObject {
    func cancel()
}

// MARK: - Production scheduler (DispatchQueue.main)

final class DispatchScheduler: BackspaceScheduler {
    func schedule(after delay: TimeInterval, block: @escaping () -> Void) -> BackspaceTask {
        let task = DispatchBackspaceTask()
        task.item = DispatchWorkItem(block: block)
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: task.item!)
        return task
    }
}

private final class DispatchBackspaceTask: BackspaceTask {
    var item: DispatchWorkItem?
    func cancel() { item?.cancel() }
}

// MARK: - BackspaceRepeater

final class BackspaceRepeater {
    var onFire: (() -> Void)?
    private(set) var hasFired = false

    private let initialDelay: TimeInterval
    private let repeatInterval: TimeInterval
    private let scheduler: BackspaceScheduler
    private var pendingTask: BackspaceTask?

    init(
        initialDelay: TimeInterval = 0.4,
        repeatInterval: TimeInterval = 0.1,
        scheduler: BackspaceScheduler = DispatchScheduler()
    ) {
        self.initialDelay = initialDelay
        self.repeatInterval = repeatInterval
        self.scheduler = scheduler
    }

    func beginHold() {
        hasFired = false
        pendingTask?.cancel()
        pendingTask = scheduler.schedule(after: initialDelay) { [weak self] in
            self?.fire()
        }
    }

    func endHold() {
        pendingTask?.cancel()
        pendingTask = nil
    }

    private func fire() {
        hasFired = true
        onFire?()
        pendingTask = scheduler.schedule(after: repeatInterval) { [weak self] in
            self?.fire()
        }
    }
}

// MARK: - BackspaceButton

final class BackspaceButton: GlassKeyButton {
    /// Fired on a quick tap (no repeat). Maps to backspaceTapped().
    var onTap: (() -> Void)?
    /// Fired on each repeat tick while the key is held. Maps to backspaceHoldFired().
    var onHoldFire: (() -> Void)?
    /// Fired once when a hold that produced repeats ends (finger up or cancelled).
    /// Maps to backspaceHoldEnded() to flush the batched session calls.
    var onHoldEnd: (() -> Void)?

    private let repeater: BackspaceRepeater

    init(repeater: BackspaceRepeater = BackspaceRepeater()) {
        self.repeater = repeater
        super.init(frame: .zero)
        repeater.onFire = { [weak self] in self?.onHoldFire?() }
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) not implemented") }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesBegan(touches, with: event)
        repeater.beginHold()
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesEnded(touches, with: event)
        if repeater.hasFired {
            onHoldEnd?()
        } else if let touch = touches.first, bounds.contains(touch.location(in: self)) {
            onTap?()
        }
        repeater.endHold()
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesCancelled(touches, with: event)
        if repeater.hasFired { onHoldEnd?() }
        repeater.endHold()
    }
}
