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

    // MARK: - helpers

    private var synchronousRunner: AnimatorRunner {
        { _, to, _, onUpdate in onUpdate(to) }
    }
}
