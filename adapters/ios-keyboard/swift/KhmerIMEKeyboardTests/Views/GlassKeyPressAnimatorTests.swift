import XCTest
@testable import KhmerIMEKeyboard

final class GlassKeyPressAnimatorTests: XCTestCase {

    // MARK: - press()

    func test_press_animatesSquishToOneWithEightyMsDuration() {
        var capturedTo: CGFloat?
        var capturedDuration: TimeInterval?

        let animator = GlassKeyPressAnimator(onUpdate: { _ in }) { _, to, duration, _ in
            capturedTo = to
            capturedDuration = duration
        }

        animator.press()

        XCTAssertEqual(capturedTo, 1)
        XCTAssertEqual(capturedDuration ?? -1, 0.080, accuracy: 0.001)
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

    func test_release_animatesSquishToZeroWithTwoHundredTwentyMsDuration() {
        var capturedTo: CGFloat?
        var capturedDuration: TimeInterval?

        let animator = GlassKeyPressAnimator(onUpdate: { _ in }) { _, to, duration, _ in
            capturedTo = to
            capturedDuration = duration
        }

        animator.release()

        XCTAssertEqual(capturedTo, 0)
        XCTAssertEqual(capturedDuration ?? -1, 0.220, accuracy: 0.001)
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
