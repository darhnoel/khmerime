import Foundation

// TextClient
// ==========
// Abstracts the IMKTextInput surface KhmerInputHandler needs, so the handler
// can be unit-tested against MockTextClient without InputMethodKit.
//
// Counterpart of TextProxy on iOS (which wraps UITextDocumentProxy). IMK
// retroactive protocol conformance is impossible for the same reason — the
// concrete client object is handed to us by the framework — so the controller
// wraps it in IMKTextClientAdapter at the call boundary.

protocol TextClient: AnyObject {
    /// Insert committed text at the insertion point.
    func insertText(_ text: String)
    /// Replace the marked (preedit) text. Empty string clears the marking.
    func setMarkedText(_ text: String)
}
