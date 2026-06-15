import XCTest
import UIKit
@testable import KhmerIMEKeyboard

final class GlassColorSpecTests: XCTestCase {

    // MARK: - backgroundColor

    func test_lightBackgroundIsTranslucent() {
        let c = GlassColorSpec.backgroundColor(isDark: false)
        XCTAssertGreaterThan(alpha(c), 0)
        XCTAssertLessThan(alpha(c), 1, "light background must be translucent")
    }

    func test_darkBackgroundIsTranslucent() {
        let c = GlassColorSpec.backgroundColor(isDark: true)
        XCTAssertGreaterThan(alpha(c), 0)
        XCTAssertLessThan(alpha(c), 1, "dark background must be translucent")
    }

    func test_darkBackgroundIsDarkerThanLight() {
        let light = GlassColorSpec.backgroundColor(isDark: false)
        let dark  = GlassColorSpec.backgroundColor(isDark: true)
        XCTAssertLessThan(luminance(dark), luminance(light),
            "dark background RGB sum must be less than light")
    }

    // MARK: - borderColor

    func test_borderColorIsTranslucent() {
        let c = GlassColorSpec.borderColor(isDark: false)
        XCTAssertGreaterThan(alpha(c), 0)
        XCTAssertLessThan(alpha(c), 1, "border must be translucent")
    }

    // MARK: - blurRadius

    func test_blurRadiusIsPositive() {
        XCTAssertGreaterThan(GlassColorSpec.blurRadius(scale: 1), 0)
    }

    func test_blurRadiusScalesLinearly() {
        let r1 = GlassColorSpec.blurRadius(scale: 1)
        let r2 = GlassColorSpec.blurRadius(scale: 2)
        XCTAssertEqual(r2, r1 * 2, accuracy: 0.01)
    }

    // MARK: - selectedCandidateBackground

    func test_selectedCandidateBackgroundIsMoreOpaqueThanRegularInLightMode() {
        let regular  = GlassColorSpec.backgroundColor(isDark: false)
        let selected = GlassColorSpec.selectedCandidateBackground(isDark: false)
        XCTAssertGreaterThan(alpha(selected), alpha(regular),
            "selected pill must be more opaque than regular key in light mode")
    }

    func test_selectedCandidateBackgroundIsMoreOpaqueThanRegularInDarkMode() {
        let regular  = GlassColorSpec.backgroundColor(isDark: true)
        let selected = GlassColorSpec.selectedCandidateBackground(isDark: true)
        XCTAssertGreaterThan(alpha(selected), alpha(regular),
            "selected pill must be more opaque than regular key in dark mode")
    }

    func test_selectedCandidateBackgroundRemainsTranslucent() {
        let c = GlassColorSpec.selectedCandidateBackground(isDark: true)
        XCTAssertLessThan(alpha(c), 1, "selected pill must stay translucent for glass effect")
    }

    // MARK: - toggleActiveBackground

    func test_toggleActiveBackgroundIsLighterThanRegularInDarkMode() {
        let regular = GlassColorSpec.backgroundColor(isDark: true)
        let active  = GlassColorSpec.toggleActiveBackground(isDark: true)
        XCTAssertGreaterThan(luminance(active), luminance(regular),
            "active toggle must be lighter than regular key in dark mode")
    }

    func test_toggleActiveBackgroundIsHighlyOpaque() {
        let c = GlassColorSpec.toggleActiveBackground(isDark: true)
        XCTAssertGreaterThanOrEqual(alpha(c), 230.0/255.0,
            "active toggle fill must be near-opaque")
    }

    // MARK: - toggleActiveTextColor

    func test_toggleActiveTextColorIsDark() {
        let c = GlassColorSpec.toggleActiveTextColor()
        XCTAssertLessThan(luminance(c), 100.0/255.0,
            "active toggle text must be dark")
    }

    func test_charPickDismissButtonUsesActiveGlassToggleStyle() {
        let panel = CandidatePanelView(metrics: KeyboardLayoutMetrics(device: .phone))

        panel.renderCharPickAlphabet()

        guard let toggle = allButtons(in: panel).first(where: { $0.title(for: .normal) == "✦" }) else {
            XCTFail("Expected char-pick alphabet to include the ✦ dismiss button")
            return
        }
        // toggleActiveBackground is near-white and near-opaque in both light and dark mode;
        // test behavioral properties rather than exact color to avoid traitCollection
        // temporal mismatch when the view is configured outside a window hierarchy.
        XCTAssertGreaterThanOrEqual(alpha(toggle.backgroundColor), 230.0/255.0,
            "active ✦ must be near-opaque")
        XCTAssertGreaterThanOrEqual(luminance(toggle.backgroundColor), 2.5,
            "active ✦ fill must be near-white in any appearance mode")
        XCTAssertEqual(luminance(toggle.titleColor(for: .normal)),
                       luminance(GlassColorSpec.toggleActiveTextColor()), accuracy: 0.01,
                       "active ✦ text must be dark")
    }

    // MARK: - candidateBorderWidth

    func test_candidateBorderWidthExceedsOnePoint() {
        XCTAssertGreaterThan(GlassColorSpec.candidateBorderWidth(), 1.0)
    }

    // MARK: - Helpers

    private func alpha(_ color: UIColor) -> CGFloat {
        var a: CGFloat = 0
        color.getRed(nil, green: nil, blue: nil, alpha: &a)
        return a
    }

    private func alpha(_ color: UIColor?) -> CGFloat {
        guard let color else { return 0 }
        return alpha(color)
    }

    private func luminance(_ color: UIColor) -> CGFloat {
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0
        color.getRed(&r, green: &g, blue: &b, alpha: nil)
        return r + g + b
    }

    private func luminance(_ color: UIColor?) -> CGFloat {
        guard let color else { return 0 }
        return luminance(color)
    }

    private func allButtons(in view: UIView) -> [UIButton] {
        var result = view.subviews.compactMap { $0 as? UIButton }
        for subview in view.subviews {
            result += allButtons(in: subview)
        }
        return result
    }
}
