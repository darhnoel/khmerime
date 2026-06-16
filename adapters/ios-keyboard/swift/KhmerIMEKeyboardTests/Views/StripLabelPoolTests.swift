import XCTest
@testable import KhmerIMEKeyboard

// StripLabelPoolTests
// ===================
// Verifies label reuse behaviour — the core invariant of Fix B.
// All tests run synchronously; UIKit label/stackview are real objects.

final class StripLabelPoolTests: XCTestCase {

    // MARK: - Growth

    func test_sync_growsPoolToRequestedCount() {
        let pool = StripLabelPool()
        let stack = UIStackView()

        pool.sync(count: 3, in: stack)

        XCTAssertEqual(pool.labels.count, 3)
        XCTAssertEqual(stack.arrangedSubviews.count, 3)
    }

    func test_sync_zeroCount_producesEmptyPool() {
        let pool = StripLabelPool()
        let stack = UIStackView()

        pool.sync(count: 0, in: stack)

        XCTAssertEqual(pool.labels.count, 0)
        XCTAssertEqual(stack.arrangedSubviews.count, 0)
    }

    // MARK: - Reuse (the key invariant)

    func test_sync_reusesLabelInstancesWhenCountIsStable() {
        let pool = StripLabelPool()
        let stack = UIStackView()

        pool.sync(count: 2, in: stack)
        let first = pool.labels[0]
        let second = pool.labels[1]

        pool.sync(count: 2, in: stack)

        XCTAssertTrue(pool.labels[0] === first,  "label[0] must be the same instance — no teardown")
        XCTAssertTrue(pool.labels[1] === second, "label[1] must be the same instance — no teardown")
    }

    func test_sync_doesNotAddArrangedSubviewsWhenCountIsStable() {
        let pool = StripLabelPool()
        let stack = UIStackView()

        pool.sync(count: 2, in: stack)
        let countBefore = stack.arrangedSubviews.count

        pool.sync(count: 2, in: stack)

        XCTAssertEqual(stack.arrangedSubviews.count, countBefore,
            "stackview must not gain arranged subviews on a stable-count sync")
    }

    // MARK: - Shrink (hide excess, keep in pool)

    func test_sync_hidesExcessLabelsWhenCountShrinks() {
        let pool = StripLabelPool()
        let stack = UIStackView()

        pool.sync(count: 3, in: stack)
        pool.sync(count: 1, in: stack)

        XCTAssertFalse(pool.labels[0].isHidden, "visible label must not be hidden")
        XCTAssertTrue(pool.labels[1].isHidden,  "excess label[1] must be hidden")
        XCTAssertTrue(pool.labels[2].isHidden,  "excess label[2] must be hidden")
    }

    func test_sync_keepsHiddenLabelsInStackViewForFutureReuse() {
        let pool = StripLabelPool()
        let stack = UIStackView()

        pool.sync(count: 3, in: stack)
        pool.sync(count: 1, in: stack)

        XCTAssertEqual(stack.arrangedSubviews.count, 3,
            "all labels remain in stack (hidden) — no removeFromSuperview")
    }

    // MARK: - Grow back (reveal hidden labels)

    func test_sync_revealsHiddenLabelsWhenCountGrowsBack() {
        let pool = StripLabelPool()
        let stack = UIStackView()

        pool.sync(count: 3, in: stack)
        pool.sync(count: 1, in: stack)   // hide 2 labels
        pool.sync(count: 3, in: stack)   // reveal them

        for (i, label) in pool.labels.enumerated() {
            XCTAssertFalse(label.isHidden, "label[\(i)] must be visible after growing back")
        }
    }

    func test_sync_doesNotAddDuplicateSubviewsWhenGrowingBack() {
        let pool = StripLabelPool()
        let stack = UIStackView()

        pool.sync(count: 3, in: stack)
        pool.sync(count: 1, in: stack)
        pool.sync(count: 3, in: stack)

        XCTAssertEqual(stack.arrangedSubviews.count, 3,
            "stack must still have exactly 3 arranged subviews — no duplicates added on grow-back")
    }

    // MARK: - Returned slice

    func test_sync_returnsOnlyVisibleLabels() {
        let pool = StripLabelPool()
        let stack = UIStackView()

        pool.sync(count: 3, in: stack)
        let visible = pool.sync(count: 2, in: stack)

        XCTAssertEqual(visible.count, 2)
        XCTAssertTrue(visible[0] === pool.labels[0])
        XCTAssertTrue(visible[1] === pool.labels[1])
    }
}
