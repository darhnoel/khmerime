import XCTest
@testable import KhmerIMEKeyboard

final class KeyboardLayerVisibilityTests: XCTestCase {

    func test_qwertyShowsOnlyQwertyLayer() {
        let visibility = KeyboardLayerVisibility(state: .qwerty)

        XCTAssertTrue(visibility.showsQwerty)
        XCTAssertFalse(visibility.showsNumeric)
        XCTAssertFalse(visibility.showsSymbols)
        XCTAssertFalse(visibility.showsPanel)
    }

    func test_numericShowsOnlyNumericLayer() {
        let visibility = KeyboardLayerVisibility(state: .numeric)

        XCTAssertFalse(visibility.showsQwerty)
        XCTAssertTrue(visibility.showsNumeric)
        XCTAssertFalse(visibility.showsSymbols)
        XCTAssertFalse(visibility.showsPanel)
    }

    func test_symbolsShowsOnlySymbolsLayer() {
        let visibility = KeyboardLayerVisibility(state: .symbols)

        XCTAssertFalse(visibility.showsQwerty)
        XCTAssertFalse(visibility.showsNumeric)
        XCTAssertTrue(visibility.showsSymbols)
        XCTAssertFalse(visibility.showsPanel)
    }

    func test_panelAndCharPickShowPanelLayer() {
        let panel = KeyboardLayerVisibility(state: .panel)
        let charPick = KeyboardLayerVisibility(state: .charPick)

        XCTAssertTrue(panel.showsPanel)
        XCTAssertTrue(charPick.showsPanel)
        XCTAssertFalse(panel.showsQwerty)
        XCTAssertFalse(charPick.showsQwerty)
    }
}
