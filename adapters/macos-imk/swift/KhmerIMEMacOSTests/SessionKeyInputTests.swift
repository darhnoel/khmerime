import XCTest
import AppKit
import Carbon.HIToolbox

// SessionKeyInputTests
// ====================
// The boundary that turns a key NSEvent into the (keyval, keycode, modifierFlags)
// tuple the Rust session consumes. Shift must be reflected in the keyval so that
// '?' (Shift+/) reaches the session as 0x3F, not the base key '/' (0x2F).

final class SessionKeyInputTests: XCTestCase {

    private func keyDown(
        keyCode: Int,
        characters: String,
        charactersIgnoringModifiers: String,
        flags: NSEvent.ModifierFlags = []
    ) -> NSEvent {
        NSEvent.keyEvent(
            with: .keyDown, location: .zero, modifierFlags: flags, timestamp: 0,
            windowNumber: 0, context: nil,
            characters: characters,
            charactersIgnoringModifiers: charactersIgnoringModifiers,
            isARepeat: false, keyCode: UInt16(keyCode)
        )!
    }

    func test_shiftSlash_yieldsQuestionMarkKeyval() {
        // Shift+/ produces '?'; the session must receive 0x3F, not '/' 0x2F.
        let event = keyDown(
            keyCode: kVK_ANSI_Slash,
            characters: "?", charactersIgnoringModifiers: "?", flags: .shift
        )
        XCTAssertEqual(sessionKeyInput(from: event).keyval, 0x3F)
    }

    func test_plainLetter_yieldsItsScalar() {
        let event = keyDown(
            keyCode: kVK_ANSI_N, characters: "n", charactersIgnoringModifiers: "n"
        )
        let input = sessionKeyInput(from: event)
        XCTAssertEqual(input.keyval, 0x6E)
        XCTAssertEqual(input.keycode, UInt16(kVK_ANSI_N))
    }
}
