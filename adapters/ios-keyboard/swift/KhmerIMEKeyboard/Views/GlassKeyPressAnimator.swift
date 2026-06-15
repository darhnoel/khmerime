import UIKit

typealias AnimatorRunner = (
    _ from: CGFloat,
    _ to: CGFloat,
    _ duration: TimeInterval,
    _ onUpdate: @escaping (CGFloat) -> Void
) -> Void

final class GlassKeyPressAnimator {

    private let onUpdate: (CGFloat) -> Void
    private let runner: AnimatorRunner
    private var squish: CGFloat = 0

    init(onUpdate: @escaping (CGFloat) -> Void, runner: AnimatorRunner? = nil) {
        self.onUpdate = onUpdate
        self.runner = runner ?? GlassKeyPressAnimator.defaultRunner()
    }

    func press() {
        runner(squish, 1, 0.080) { [weak self] v in
            self?.squish = v
            self?.onUpdate(v)
        }
    }

    func release() {
        runner(squish, 0, 0.220) { [weak self] v in
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
