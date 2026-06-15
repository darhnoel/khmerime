import XCTest
import UIKit
@testable import KhmerIMEKeyboard

final class GlassKeyButtonTests: XCTestCase {

    // MARK: - press squish

    func test_touchesBegan_squishesButtonToNinetyTwoPercent() {
        let btn = GlassKeyButton()
        btn.configureForTesting(runner: synchronousRunner)

        btn.touchesBegan(Set(), with: nil)

        XCTAssertEqual(btn.transform.a, 0.92, accuracy: 0.01,
            "full press must shrink button to 92% (1 - 0.08)")
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

    func test_inactive_backgroundIsTransparent() {
        let btn = GlassKeyButton()
        btn.frame = CGRect(x: 0, y: 0, width: 40, height: 44)
        btn.layoutIfNeeded()

        var alpha: CGFloat = 0
        (btn.backgroundColor ?? .clear).getRed(nil, green: nil, blue: nil, alpha: &alpha)
        XCTAssertEqual(alpha, 0, accuracy: 0.01,
            "inactive button background must be transparent — UIVisualEffectView provides the glass depth")
    }

    // MARK: - helpers

    private var synchronousRunner: AnimatorRunner {
        { _, to, _, onUpdate in onUpdate(to) }
    }
}
