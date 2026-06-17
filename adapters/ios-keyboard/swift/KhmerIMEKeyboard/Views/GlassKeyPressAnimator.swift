import UIKit

typealias AnimatorRunner = (
    _ from: CGFloat,
    _ to: CGFloat,
    _ duration: TimeInterval,
    _ onUpdate: @escaping (CGFloat) -> Void
) -> Void

final class GlassKeyPressAnimator {

    // Press/release durations are deliberately short. During fast typing the
    // release must finish before the next keystroke, otherwise the grow-back
    // animation trails the finger and the keyboard feels sluggish even though
    // every tap registers. Press is near-instant for immediate tactile feedback.
    static let pressDuration: TimeInterval = 0.040
    static let releaseDuration: TimeInterval = 0.090

    private let onUpdate: (CGFloat) -> Void
    private let runner: AnimatorRunner
    private var squish: CGFloat = 0

    init(onUpdate: @escaping (CGFloat) -> Void, runner: AnimatorRunner? = nil) {
        self.onUpdate = onUpdate
        self.runner = runner ?? GlassKeyPressAnimator.defaultRunner()
    }

    func press() {
        runner(squish, 1, Self.pressDuration) { [weak self] v in
            self?.squish = v
            self?.onUpdate(v)
        }
    }

    func release() {
        runner(squish, 0, Self.releaseDuration) { [weak self] v in
            self?.squish = v
            self?.onUpdate(v)
        }
    }

    private static func defaultRunner() -> AnimatorRunner {
        { _, to, duration, onUpdate in
            UIView.animate(
                withDuration: duration,
                delay: 0,
                options: [.curveEaseOut, .beginFromCurrentState],
                animations: { onUpdate(to) }
            )
        }
    }
}
