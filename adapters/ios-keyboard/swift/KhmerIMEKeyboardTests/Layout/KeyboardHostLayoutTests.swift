import UIKit
import XCTest

final class KeyboardHostLayoutTests: XCTestCase {
    func test_installPinsRootToHostAndReturnsActiveHeightConstraint() {
        let host = UIView()
        let root = UIView()
        let metrics = KeyboardLayoutMetrics(device: .phone)

        let heightConstraint = KeyboardHostLayout.install(rootView: root, in: host, metrics: metrics, safeAreaBottom: 12)

        XCTAssertTrue(host.subviews.contains(root))
        XCTAssertFalse(root.translatesAutoresizingMaskIntoConstraints)
        XCTAssertEqual(host.backgroundColor, UIColor.systemGray5)
        XCTAssertTrue(heightConstraint.isActive)
        XCTAssertEqual(heightConstraint.priority, UILayoutPriority(999))
        XCTAssertEqual(heightConstraint.constant, metrics.baseKeyboardHeight + 12)
        XCTAssertGreaterThanOrEqual(host.constraints.filter(\.isActive).count, 5)
    }

    func test_heightConstantAddsSafeAreaBottomToBaseKeyboardHeight() {
        let metrics = KeyboardLayoutMetrics(device: .pad)

        XCTAssertEqual(
            KeyboardHostLayout.heightConstant(metrics: metrics, safeAreaBottom: 34),
            metrics.baseKeyboardHeight + 34
        )
    }
}
