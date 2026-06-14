import XCTest
import AppKit
import Carbon.HIToolbox

// SessionKeyInputTests
// ====================
// The boundary that turns a key NSEvent into the (keyval, keycode, modifierFlags)
// tuple the Rust session consumes. sessionKeyInput uses event.characters (not
// charactersIgnoringModifiers) because macOS strips Shift from symbol keys in
// charactersIgnoringModifiers — Shift+/ gives "/" not "?", Shift+1 gives "1" not "!".

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
        // macOS sets charactersIgnoringModifiers="/" for Shift+/ (Shift stripped from symbols).
        // sessionKeyInput must use event.characters="?" instead.
        let event = keyDown(
            keyCode: kVK_ANSI_Slash,
            characters: "?", charactersIgnoringModifiers: "/", flags: .shift
        )
        XCTAssertEqual(sessionKeyInput(from: event).keyval, 0x3F)
    }

    func test_shiftDigit_yieldsShiftedCharKeyval() {
        // macOS sets charactersIgnoringModifiers="1" for Shift+1 (Shift stripped from symbols).
        // sessionKeyInput must use event.characters="!" instead.
        let event = keyDown(
            keyCode: kVK_ANSI_1,
            characters: "!", charactersIgnoringModifiers: "1", flags: .shift
        )
        XCTAssertEqual(sessionKeyInput(from: event).keyval, UInt32(("!" as UnicodeScalar).value))
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
