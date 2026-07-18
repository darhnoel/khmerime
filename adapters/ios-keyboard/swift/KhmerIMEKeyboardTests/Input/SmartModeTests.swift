import XCTest
@testable import KhmerIMEKeyboard

// SmartModeTests
// ==============
// The testable core of the Standard/Smart toggle: the KeyboardSession.setModelMode
// wrapper over the Rust seam, and SmartModePreference persistence. Mirrors the Android
// SmartModePreference semantics. Integration-style — real KeyboardSession, real
// UserDefaults suite (injected, throwaway, so tests never touch the shared group suite).

final class SmartModeTests: XCTestCase {

    // MARK: - KeyboardSession.setModelMode round-trip (tracer)

    func test_setModelMode_togglesModelMode() {
        let session = KeyboardSession()
        XCTAssertFalse(session.isModelMode(), "fresh session must default to Standard")

        session.setModelMode(true)
        XCTAssertTrue(session.isModelMode(), "setModelMode(true) must enable Smart")

        session.setModelMode(false)
        XCTAssertFalse(session.isModelMode(), "setModelMode(false) must return to Standard")
    }

    // MARK: - SmartModePreference persistence

    // A throwaway suite so tests never touch the real App Group.
    private func makeDefaults() -> UserDefaults {
        let suite = "test.smartmode.\(UUID().uuidString)"
        return UserDefaults(suiteName: suite)!
    }

    func test_isEnabled_defaultsToFalse() {
        let prefs = SmartModePreference(defaults: makeDefaults())
        XCTAssertFalse(prefs.isEnabled, "Smart mode must default to Standard (off)")
    }

    func test_setEnabled_persistsTheChoice() {
        let prefs = SmartModePreference(defaults: makeDefaults())
        prefs.setEnabled(true)
        XCTAssertTrue(prefs.isEnabled, "setEnabled(true) must persist so isEnabled returns true")
        prefs.setEnabled(false)
        XCTAssertFalse(prefs.isEnabled, "setEnabled(false) must return to Standard")
    }
}
