import Carbon.HIToolbox
import XCTest

// KeyvalMappingTests
// ==================
// The keyval function must produce the same X11 keysym constants the Linux
// IBus adapter delivers — the Rust session dispatches on these exact values.

final class KeyvalMappingTests: XCTestCase {

    func test_specialKeys_mapToX11Keysyms() {
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_Return), unmodifiedCharacters: "\r"), 0xFF0D)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_ANSI_KeypadEnter), unmodifiedCharacters: nil), 0xFF0D)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_Delete), unmodifiedCharacters: nil), 0xFF08)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_Tab), unmodifiedCharacters: "\t"), 0xFF09)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_Escape), unmodifiedCharacters: nil), 0xFF1B)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_Space), unmodifiedCharacters: " "), 0x0020)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_LeftArrow), unmodifiedCharacters: nil), 0xFF51)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_RightArrow), unmodifiedCharacters: nil), 0xFF53)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_UpArrow), unmodifiedCharacters: nil), 0xFF52)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_DownArrow), unmodifiedCharacters: nil), 0xFF54)
    }

    func test_printableKeys_useUnicodeScalar() {
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_ANSI_A), unmodifiedCharacters: "a"), 97)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_ANSI_A), unmodifiedCharacters: "A"), 65)
        XCTAssertEqual(keyval(forMacKeyCode: UInt16(kVK_ANSI_1), unmodifiedCharacters: "1"), 0x31)
    }

    func test_unknownKeyWithNoCharacters_returnsZero() {
        XCTAssertEqual(keyval(forMacKeyCode: 0xFFFF, unmodifiedCharacters: nil), 0)
    }
}
