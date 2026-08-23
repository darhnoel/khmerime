import XCTest
@testable import KhmerIMEKeyboard

/// The next-keyboard globe must be present on every device, regardless of
/// `needsInputModeSwitchKey` (App Store Guideline 4.4.1 — see ADR-0022). The
/// old "Option B" hid the globe and showed EN instead when the system hint was
/// false, which is exactly what Apple rejected.
final class SwitchKeyVisibilityTests: XCTestCase {

    func test_globeAlwaysShown_whenSystemOffersSwitching() {
        let v = SwitchKeyVisibility(needsInputModeSwitchKey: true)
        XCTAssertFalse(v.globeHidden)
    }

    func test_globeAlwaysShown_whenSystemDoesNotOfferSwitching() {
        // iPad / iPhone X-class: the system hint is false, but the globe MUST
        // still show — this is the rejection case.
        let v = SwitchKeyVisibility(needsInputModeSwitchKey: false)
        XCTAssertFalse(v.globeHidden)
    }

    func test_englishToggleAlwaysShown() {
        // EN is a distinct feature (the in-keyboard English-layer toggle), not a
        // keyboard switcher; it stays visible in both cases.
        XCTAssertFalse(SwitchKeyVisibility(needsInputModeSwitchKey: true).englishHidden)
        XCTAssertFalse(SwitchKeyVisibility(needsInputModeSwitchKey: false).englishHidden)
    }
}
