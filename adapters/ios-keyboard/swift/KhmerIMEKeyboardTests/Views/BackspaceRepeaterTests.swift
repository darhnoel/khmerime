import XCTest
@testable import KhmerIMEKeyboard

// BackspaceRepeaterTests
// ======================
// Tests BackspaceRepeater behavior through ManualScheduler — no real timers,
// no XCTestExpectation. Each firePending() call advances time by one scheduled
// task, letting tests drive the hold/repeat cycle deterministically.

final class BackspaceRepeaterTests: XCTestCase {

    // MARK: - Behavior 1: endHold before initial delay prevents any fire

    func test_endHold_beforeInitialDelay_doesNotFire() {
        let scheduler = ManualScheduler()
        let repeater = BackspaceRepeater(scheduler: scheduler)
        var fired = false
        repeater.onFire = { fired = true }

        repeater.beginHold()
        repeater.endHold()
        scheduler.firePending()   // cancelled task — must not run block

        XCTAssertFalse(fired)
    }

    // MARK: - Behavior 2: initial delay fires onFire once

    func test_beginHold_afterInitialDelay_firesOnce() {
        let scheduler = ManualScheduler()
        let repeater = BackspaceRepeater(scheduler: scheduler)
        var count = 0
        repeater.onFire = { count += 1 }

        repeater.beginHold()
        scheduler.firePending()   // initial delay → first fire

        XCTAssertEqual(count, 1)
    }

    // MARK: - Behavior 3: hasFired reflects fire state

    func test_hasFired_isFalseBeforeInitialDelay() {
        let scheduler = ManualScheduler()
        let repeater = BackspaceRepeater(scheduler: scheduler)

        repeater.beginHold()

        XCTAssertFalse(repeater.hasFired)
    }

    func test_hasFired_isTrueAfterInitialDelay() {
        let scheduler = ManualScheduler()
        let repeater = BackspaceRepeater(scheduler: scheduler)

        repeater.beginHold()
        scheduler.firePending()

        XCTAssertTrue(repeater.hasFired)
    }

    // MARK: - Behavior 4: repeat interval schedules subsequent fires

    func test_firePendingTwice_callsOnFireTwice() {
        let scheduler = ManualScheduler()
        let repeater = BackspaceRepeater(scheduler: scheduler)
        var count = 0
        repeater.onFire = { count += 1 }

        repeater.beginHold()
        scheduler.firePending()   // initial delay → first fire
        scheduler.firePending()   // repeat interval → second fire

        XCTAssertEqual(count, 2)
    }

    func test_firePendingFiveTimes_callsOnFireFiveTimes() {
        let scheduler = ManualScheduler()
        let repeater = BackspaceRepeater(scheduler: scheduler)
        var count = 0
        repeater.onFire = { count += 1 }

        repeater.beginHold()
        for _ in 0..<5 { scheduler.firePending() }

        XCTAssertEqual(count, 5)
    }

    // MARK: - Behavior 5: endHold after first fire stops repeat

    func test_endHold_afterFirstFire_preventsSecondFire() {
        let scheduler = ManualScheduler()
        let repeater = BackspaceRepeater(scheduler: scheduler)
        var count = 0
        repeater.onFire = { count += 1 }

        repeater.beginHold()
        scheduler.firePending()   // first fire
        repeater.endHold()        // cancel pending repeat
        scheduler.firePending()   // no-op — cancelled

        XCTAssertEqual(count, 1)
    }

    // MARK: - Behavior 6: beginHold resets hasFired for a new hold cycle

    func test_beginHold_resetsHasFired() {
        let scheduler = ManualScheduler()
        let repeater = BackspaceRepeater(scheduler: scheduler)

        repeater.beginHold()
        scheduler.firePending()   // hasFired = true
        XCTAssertTrue(repeater.hasFired)

        repeater.beginHold()      // starts new hold cycle
        XCTAssertFalse(repeater.hasFired, "beginHold must reset hasFired")
    }

    // MARK: - Behavior 7: second beginHold cancels the first pending task

    func test_secondBeginHold_cancelsFirstPendingTask() {
        let scheduler = ManualScheduler()
        let repeater = BackspaceRepeater(scheduler: scheduler)
        var count = 0
        repeater.onFire = { count += 1 }

        repeater.beginHold()
        repeater.beginHold()      // overwrites the first scheduled task
        scheduler.firePending()   // only one task remains

        XCTAssertEqual(count, 1)
    }
}

// MARK: - Test Doubles

private final class ManualTask: BackspaceTask {
    private(set) var isCancelled = false
    let block: () -> Void
    init(block: @escaping () -> Void) { self.block = block }
    func cancel() { isCancelled = true }
}

final class ManualScheduler: BackspaceScheduler {
    private var pending: [ManualTask] = []

    func schedule(after _: TimeInterval, block: @escaping () -> Void) -> BackspaceTask {
        let task = ManualTask(block: block)
        pending.append(task)
        return task
    }

    /// Fires the first non-cancelled pending task. Returns true if one was found.
    @discardableResult
    func firePending() -> Bool {
        guard let idx = pending.firstIndex(where: { !$0.isCancelled }) else { return false }
        let task = pending.remove(at: idx)
        task.block()
        return true
    }
}
