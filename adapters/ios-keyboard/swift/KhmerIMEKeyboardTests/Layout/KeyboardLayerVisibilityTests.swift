import XCTest
@testable import KhmerIMEKeyboard

final class KeyboardLayerVisibilityTests: XCTestCase {

    func test_qwertyShowsOnlyQwertyLayer() {
        let visibility = KeyboardLayerVisibility(state: .qwerty)

        XCTAssertTrue(visibility.showsQwerty)
        XCTAssertFalse(visibility.showsNumeric)
        XCTAssertFalse(visibility.showsSymbols)
        XCTAssertTrue(visibility.showsCandidateRow)
    }

    func test_numericShowsOnlyNumericLayer() {
        let visibility = KeyboardLayerVisibility(state: .numeric)

        XCTAssertFalse(visibility.showsQwerty)
        XCTAssertTrue(visibility.showsNumeric)
        XCTAssertFalse(visibility.showsSymbols)
        XCTAssertTrue(visibility.showsCandidateRow)
    }

    func test_symbolsShowsOnlySymbolsLayer() {
        let visibility = KeyboardLayerVisibility(state: .symbols)

        XCTAssertFalse(visibility.showsQwerty)
        XCTAssertFalse(visibility.showsNumeric)
        XCTAssertTrue(visibility.showsSymbols)
        XCTAssertTrue(visibility.showsCandidateRow)
    }

    func test_charPickShowsQwertyLayerAndCandidateRow() {
        let visibility = KeyboardLayerVisibility(state: .charPick)

        XCTAssertTrue(visibility.showsQwerty)
        XCTAssertFalse(visibility.showsNumeric)
        XCTAssertFalse(visibility.showsSymbols)
        XCTAssertTrue(visibility.showsCandidateRow)
    }
}
