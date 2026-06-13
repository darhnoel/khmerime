import AppKit
import Carbon.HIToolbox
import Foundation

// KeyvalMapping
// =============
// Converts a mac key event (keyCode + unmodified characters) to an XKB keyval.
//
// For printable ASCII keys the Unicode scalar is the keyval.
// For special keys (arrows, Return, Backspace, Tab, Escape) we map to X11
// keysym constants matching what the Linux IBus adapter delivers — the Rust
// session uses these same constants (KEY_RETURN = 0xFF0D, etc.).
//
// Takes primitives instead of NSEvent so it is testable without synthesizing
// AppKit events; KhmerInputController extracts the primitives at the boundary.

func keyval(forMacKeyCode keyCode: UInt16, unmodifiedCharacters: String?) -> UInt32 {
    switch Int(keyCode) {
    case kVK_Return, kVK_ANSI_KeypadEnter: return 0xFF0D
    case kVK_Delete:                        return 0xFF08
    case kVK_Tab:                           return 0xFF09
    case kVK_Escape:                        return 0xFF1B
    case kVK_Space:                         return 0x0020
    case kVK_LeftArrow:                     return 0xFF51
    case kVK_RightArrow:                    return 0xFF53
    case kVK_UpArrow:                       return 0xFF52
    case kVK_DownArrow:                     return 0xFF54
    default:
        return unmodifiedCharacters?.unicodeScalars.first?.value ?? 0
    }
}

// Derives the (keyval, keycode, modifierFlags) the Rust session consumes from a
// key NSEvent. Uses `charactersIgnoringModifiers`, which reflects Shift/CapsLock
// but ignores Command/Control/Option — so '?' (Shift+/) becomes keyval 0x3F and
// 'N' (Shift+n) becomes 0x4E, while Cmd/Ctrl/Option stay modifier flags instead
// of turning the keyval into a control code or accented character. (Using the
// base, shift-stripped character here would lose '?', uppercase, and every other
// shifted key.)
func sessionKeyInput(from event: NSEvent) -> (keyval: UInt32, keycode: UInt16, modifierFlags: UInt32) {
    let keyCode = UInt16(event.keyCode)
    return (
        keyval: keyval(
            forMacKeyCode: keyCode,
            unmodifiedCharacters: event.charactersIgnoringModifiers ?? event.characters
        ),
        keycode: keyCode,
        modifierFlags: UInt32(event.modifierFlags.rawValue)
    )
}
