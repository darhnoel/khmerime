import XCTest
@testable import KhmerIMEKeyboard

final class GlassKeyPressAnimatorTests: XCTestCase {

    // MARK: - press()

    func test_press_animatesSquishToOneWithSnappyDuration() {
        var capturedTo: CGFloat?
        var capturedDuration: TimeInterval?

        let animator = GlassKeyPressAnimator(onUpdate: { _ in }) { _, to, duration, _ in
            capturedTo = to
            capturedDuration = duration
        }

        animator.press()

        XCTAssertEqual(capturedTo, 1)
        // Press feedback must be near-instant so the squish keeps up with fast typing.
        XCTAssertEqual(capturedDuration ?? -1, 0.040, accuracy: 0.001)
    }

    func test_press_startsFromZeroOnFirstPress() {
        var capturedFrom: CGFloat?

        let animator = GlassKeyPressAnimator(onUpdate: { _ in }) { from, _, _, _ in
            capturedFrom = from
        }

        animator.press()

        XCTAssertEqual(capturedFrom, 0)
    }

    func test_press_callsOnUpdateWithSquishValue() {
        var updatedValue: CGFloat?
        let animator = GlassKeyPressAnimator(onUpdate: { v in updatedValue = v }) { _, to, _, onUpdate in
            onUpdate(to)
        }

        animator.press()

        XCTAssertEqual(updatedValue, 1)
    }

    // MARK: - release()

    func test_release_animatesSquishToZeroWithSnappyDuration() {
        var capturedTo: CGFloat?
        var capturedDuration: TimeInterval?

        let animator = GlassKeyPressAnimator(onUpdate: { _ in }) { _, to, duration, _ in
            capturedTo = to
            capturedDuration = duration
        }

        animator.release()

        XCTAssertEqual(capturedTo, 0)
        // Release must be fast enough to finish before the next keystroke during
        // fast typing — a slow grow-back is what makes keys feel like they trail
        // the finger. 90ms keeps the squish from lagging behind rapid taps.
        XCTAssertEqual(capturedDuration ?? -1, 0.090, accuracy: 0.001)
    }

    // MARK: - interruptible mid-flight

    func test_release_afterPartialPress_startsFromCurrentSquish() {
        var capturedFrom: CGFloat?

        let animator = GlassKeyPressAnimator(onUpdate: { _ in }) { from, to, _, onUpdate in
            capturedFrom = from
            onUpdate((from + to) / 2)
        }

        animator.press()
        animator.release()

        XCTAssertEqual(capturedFrom ?? -1, 0.5, accuracy: 0.01,
            "release must start from current squish, not from 0")
    }
}
