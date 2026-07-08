import UIKit
import XCTest
@testable import KhmerIMEKeyboard

final class KeyboardResourceTeardownTests: XCTestCase {
    func test_releaseInteractionsClearsCallbacksTargetsAndGesturesRecursively() {
        let target = TeardownTarget()
        let root = UIView()
        let nested = UIView()
        root.addSubview(nested)

        let key = GlassKeyButton()
        key.onPress = {}
        key.onPreviewChanged = { _, _ in }
        key.addTarget(target, action: #selector(TeardownTarget.action), for: .touchUpInside)
        nested.addSubview(key)

        let backspace = BackspaceButton()
        backspace.onTap = {}
        backspace.onHoldFire = {}
        backspace.onHoldEnd = {}
        nested.addSubview(backspace)

        let globe = GlobeKeyButton()
        globe.onShortTap = {}
        globe.onLongPress = { _, _ in }
        nested.addSubview(globe)

        let gestureHost = UIView()
        gestureHost.addGestureRecognizer(UITapGestureRecognizer(target: target, action: #selector(TeardownTarget.action)))
        nested.addSubview(gestureHost)

        let strip = StripView()
        strip.onKhmerRowTapped = {}
        strip.onKhmerRowLongPressed = {}
        strip.onSegmentFocused = { _ in }
        nested.addSubview(strip)

        let candidateRow = CandidateRowView()
        candidateRow.onCandidateSelected = { _ in }
        nested.addSubview(candidateRow)

        KeyboardResourceTeardown.releaseInteractions(in: root)

        XCTAssertNil(key.onPress)
        XCTAssertNil(key.onPreviewChanged)
        XCTAssertNil(key.actions(forTarget: target, forControlEvent: .touchUpInside))
        XCTAssertNil(backspace.onTap)
        XCTAssertNil(backspace.onHoldFire)
        XCTAssertNil(backspace.onHoldEnd)
        XCTAssertNil(globe.onShortTap)
        XCTAssertNil(globe.onLongPress)
        XCTAssertTrue(gestureHost.gestureRecognizers?.isEmpty ?? true)
        XCTAssertNil(strip.onKhmerRowTapped)
        XCTAssertNil(strip.onKhmerRowLongPressed)
        XCTAssertNil(strip.onSegmentFocused)
        XCTAssertNil(candidateRow.onCandidateSelected)
    }
}

private final class TeardownTarget: NSObject {
    @objc func action() {}
}
