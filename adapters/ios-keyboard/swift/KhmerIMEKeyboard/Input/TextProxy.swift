import UIKit

// TextProxy
// =========
// The three UITextDocumentProxy operations KeyboardInputHandler needs.
// Keeping this to a minimal protocol lets tests inject a MockTextProxy
// instead of a real UITextDocumentProxy (which requires a running
// keyboard extension process).

protocol TextProxy: AnyObject {
    func insertText(_ text: String)
    func deleteBackward()
    var documentContextBeforeInput: String? { get }
}

// UITextDocumentProxy is itself a protocol, so we can't conform it directly.
// DocumentProxyWrapper bridges the real proxy to TextProxy for the handler.
final class DocumentProxyWrapper: TextProxy {
    private let inner: UITextDocumentProxy
    init(_ proxy: UITextDocumentProxy) { inner = proxy }
    func insertText(_ text: String) { inner.insertText(text) }
    func deleteBackward() { inner.deleteBackward() }
    var documentContextBeforeInput: String? { inner.documentContextBeforeInput }
}
