import XCTest
import UIKit
@testable import KhmerIMEKeyboard

final class GlassKeyButtonTests: XCTestCase {

    // MARK: - rapid-tap registration

    // Verifies that every touchesBegan fires onPress, regardless of whether
    // a previous touch is still active. UIKit only delivers overlapping touches
    // to a view when isMultipleTouchEnabled = true; without it the second tap
    // is silently dropped by UIKit before touchesBegan is even called.
    func test_isMultipleTouchEnabled_allowsEveryTouchToFireOnPress() {
        let btn = GlassKeyButton()
        XCTAssertTrue(btn.isMultipleTouchEnabled,
            "isMultipleTouchEnabled must be true — rapid taps where the second " +
            "touch begins before the first ends are silently dropped otherwise")
    }

    func test_secondTouchesBegan_whileFirstStillActive_firesOnPressAgain() {
        let btn = GlassKeyButton()
        btn.configureForTesting(runner: synchronousRunner)
        var count = 0
        btn.onPress = { count += 1 }

        btn.touchesBegan(Set(), with: nil)   // first touch down
        btn.touchesBegan(Set(), with: nil)   // second touch before first lifts

        XCTAssertEqual(count, 2,
            "each touchesBegan must fire onPress — rapid overlapping taps must both register")
    }

    // The press animation scales the button to 92%. UIKit hit-tests through the
    // scaled geometry, so a re-tap landing in the outer "dead ring" (inside the
    // layout frame but outside the shrunken visual) is routed past the button and
    // touchesBegan never fires. This is why every other rapid tap is dropped.
    // The button must claim touches across its full un-squished frame.
    func test_squishedButton_stillReceivesTouchInOuterRing() {
        let container = UIView(frame: CGRect(x: 0, y: 0, width: 100, height: 100))
        let btn = GlassKeyButton(frame: CGRect(x: 0, y: 0, width: 40, height: 44))
        container.addSubview(btn)
        // Simulate the mid-press squished state (release animation still running).
        btn.transform = CGAffineTransform(scaleX: 0.92, y: 0.92)

        // A point inside the layout frame (x < 40) but outside the 92%-scaled
        // visual — exactly where a fast re-tap is currently dropped.
        let deadRingPoint = CGPoint(x: 39, y: 22)
        let hit = container.hitTest(deadRingPoint, with: nil)

        XCTAssertTrue(hit === btn,
            "a rapid re-tap landing in the squished button's outer ring must still hit the button — " +
            "otherwise every other fast tap is silently dropped")
    }

    // MARK: - press squish

    func test_touchesBegan_squishesButtonToNinetyTwoPercent() {
        let btn = GlassKeyButton()
        btn.configureForTesting(runner: synchronousRunner)

        btn.touchesBegan(Set(), with: nil)

        XCTAssertEqual(btn.transform.a, 0.92, accuracy: 0.01,
            "full press must shrink button to 92% (1 - 0.08)")
    }

    // MARK: - key preview popup events

    func test_previewableKey_showsPreviewOnTouchBeganAndHidesOnTouchEnded() {
        let btn = GlassKeyButton()
        btn.configureForTesting(runner: synchronousRunner)
        btn.previewLabel = "A"
        var events: [(keyIsSource: Bool, label: String?)] = []
        btn.onPreviewChanged = { key, label in
            events.append((key === btn, label))
        }

        btn.touchesBegan(Set(), with: nil)
        btn.touchesEnded(Set(), with: nil)

        XCTAssertEqual(events.count, 2)
        XCTAssertTrue(events[0].keyIsSource)
        XCTAssertEqual(events[0].label, "A")
        XCTAssertTrue(events[1].keyIsSource)
        XCTAssertNil(events[1].label)
    }

    func test_previewableKey_hidesPreviewOnTouchCancelled() {
        let btn = GlassKeyButton()
        btn.configureForTesting(runner: synchronousRunner)
        btn.previewLabel = "A"
        var labels: [String?] = []
        btn.onPreviewChanged = { _, label in labels.append(label) }

        btn.touchesBegan(Set(), with: nil)
        btn.touchesCancelled(Set(), with: nil)

        XCTAssertEqual(labels.count, 2)
        XCTAssertEqual(labels[0], "A")
        XCTAssertNil(labels[1])
    }

    func test_nonPreviewableKeyDoesNotEmitPreviewEvents() {
        let btn = GlassKeyButton()
        btn.configureForTesting(runner: synchronousRunner)
        var labels: [String?] = []
        btn.onPreviewChanged = { _, label in labels.append(label) }

        btn.touchesBegan(Set(), with: nil)
        btn.touchesEnded(Set(), with: nil)

        XCTAssertTrue(labels.isEmpty)
    }

    // MARK: - release restore

    func test_touchesEnded_restoresTransformToIdentity() {
        let btn = GlassKeyButton()
        btn.configureForTesting(runner: synchronousRunner)

        btn.touchesBegan(Set(), with: nil)
        btn.touchesEnded(Set(), with: nil)

        XCTAssertEqual(btn.transform.a, 1.0, accuracy: 0.001,
            "release must restore button scale to 1.0")
    }

    func test_touchesCancelled_restoresTransformToIdentity() {
        let btn = GlassKeyButton()
        btn.configureForTesting(runner: synchronousRunner)

        btn.touchesBegan(Set(), with: nil)
        btn.touchesCancelled(Set(), with: nil)

        XCTAssertEqual(btn.transform.a, 1.0, accuracy: 0.001,
            "cancelled touch must restore button scale to 1.0")
    }

    // MARK: - real glass blur

    func test_hasBlurViewSubview() {
        let btn = GlassKeyButton()
        btn.frame = CGRect(x: 0, y: 0, width: 40, height: 44)
        btn.layoutIfNeeded()

        XCTAssertNotNil(
            btn.subviews.first { $0 is UIVisualEffectView },
            "GlassKeyButton must contain a UIVisualEffectView — real Metal blur, not flat backgroundColor"
        )
    }

    func test_blurView_fillsBoundsAfterLayout() {
        let btn = GlassKeyButton()
        btn.frame = CGRect(x: 0, y: 0, width: 40, height: 44)
        btn.layoutIfNeeded()

        let blurView = btn.subviews.compactMap { $0 as? UIVisualEffectView }.first!
        XCTAssertEqual(blurView.frame, btn.bounds)
    }

    func test_blurView_cornerRadiusMatchesGlassProportion() {
        let btn = GlassKeyButton()
        btn.frame = CGRect(x: 0, y: 0, width: 40, height: 44)
        btn.layoutIfNeeded()

        let blurView = btn.subviews.compactMap { $0 as? UIVisualEffectView }.first!
        let expected = GlassColorSpec.keyCornerRadius(height: 44)
        XCTAssertEqual(blurView.layer.cornerRadius, expected, accuracy: 0.01,
            "blur view corner radius must follow glass proportion (height * 0.22)")
    }

    // MARK: - press color feedback

    func test_touchesBegan_setsBackgroundToPressedColor() {
        let btn = GlassKeyButton()
        btn.frame = CGRect(x: 0, y: 0, width: 40, height: 44)
        btn.layoutIfNeeded()

        btn.touchesBegan(Set(), with: nil)

        var alpha: CGFloat = 0
        (btn.backgroundColor ?? .clear).getRed(nil, green: nil, blue: nil, alpha: &alpha)
        XCTAssertGreaterThanOrEqual(alpha, 200.0/255.0,
            "pressed key must show a clearly visible tint")
    }

    func test_touchesEnded_revertsBackgroundToTransparentWhenInactive() {
        let btn = GlassKeyButton()
        btn.frame = CGRect(x: 0, y: 0, width: 40, height: 44)
        btn.layoutIfNeeded()

        btn.touchesBegan(Set(), with: nil)
        btn.touchesEnded(Set(), with: nil)

        var alpha: CGFloat = 0
        (btn.backgroundColor ?? .clear).getRed(nil, green: nil, blue: nil, alpha: &alpha)
        XCTAssertEqual(alpha, 0, accuracy: 0.01,
            "releasing an inactive key must restore transparency")
    }

    func test_touchesCancelled_revertsBackgroundToTransparentWhenInactive() {
        let btn = GlassKeyButton()
        btn.frame = CGRect(x: 0, y: 0, width: 40, height: 44)
        btn.layoutIfNeeded()

        btn.touchesBegan(Set(), with: nil)
        btn.touchesCancelled(Set(), with: nil)

        var alpha: CGFloat = 0
        (btn.backgroundColor ?? .clear).getRed(nil, green: nil, blue: nil, alpha: &alpha)
        XCTAssertEqual(alpha, 0, accuracy: 0.01,
            "cancelling touch on an inactive key must restore transparency")
    }

    func test_touchesEnded_revertsBackgroundToActiveColorWhenGlassActive() {
        let btn = GlassKeyButton()
        btn.frame = CGRect(x: 0, y: 0, width: 40, height: 44)
        btn.isGlassActive = true
        btn.layoutIfNeeded()

        btn.touchesBegan(Set(), with: nil)
        btn.touchesEnded(Set(), with: nil)

        var alpha: CGFloat = 0
        (btn.backgroundColor ?? .clear).getRed(nil, green: nil, blue: nil, alpha: &alpha)
        XCTAssertGreaterThanOrEqual(alpha, 230.0/255.0,
            "releasing an active key (e.g. EN/✦) must restore its near-opaque active fill, not transparency")
    }

    func test_inactive_backgroundIsTransparent() {
        let btn = GlassKeyButton()
        btn.frame = CGRect(x: 0, y: 0, width: 40, height: 44)
        btn.layoutIfNeeded()

        var alpha: CGFloat = 0
        (btn.backgroundColor ?? .clear).getRed(nil, green: nil, blue: nil, alpha: &alpha)
        XCTAssertEqual(alpha, 0, accuracy: 0.01,
            "inactive button background must be transparent — UIVisualEffectView provides the glass depth")
    }

    // MARK: - lifetime

    func test_pressAnimatorDoesNotRetainButton() {
        weak var weakButton: GlassKeyButton?

        autoreleasepool {
            var button: GlassKeyButton? = GlassKeyButton()
            button?.configureForTesting(runner: synchronousRunner)
            button?.touchesBegan(Set(), with: nil)
            weakButton = button
            button = nil
        }

        XCTAssertNil(weakButton,
            "GlassKeyButton must not be retained by its press animator closure")
    }

    // MARK: - helpers

    private var synchronousRunner: AnimatorRunner {
        { _, to, _, onUpdate in onUpdate(to) }
    }
}
