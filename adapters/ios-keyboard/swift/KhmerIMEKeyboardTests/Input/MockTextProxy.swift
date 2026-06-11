@testable import KhmerIMEKeyboard

// MockTextProxy
// =============
// Simulates a text field as a plain String. Tests assert on `text` to
// verify exactly what the keyboard inserted and deleted.
//
// autoSpaceAfterInsert: set to true to simulate the iOS autocorrect behavior
// where insertText() after deleteBackward()×N appends a trailing space.

final class MockTextProxy: TextProxy {

    var text = ""
    var autoSpaceAfterInsert = false
    private var pendingDeletes = 0

    func insertText(_ s: String) {
        // iOS only appends the autocorrect space for word replacements, not for
        // newlines, spaces, or other non-word inserts.
        if autoSpaceAfterInsert && pendingDeletes > 0 && !s.hasSuffix("\n") && s != " " {
            text += s + " "
        } else {
            text += s
        }
        pendingDeletes = 0
    }

    func deleteBackward() {
        guard !text.isEmpty else { return }
        text.removeLast()
        pendingDeletes += 1
    }

    var documentContextBeforeInput: String? { text }
}
