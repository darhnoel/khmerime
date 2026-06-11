import Foundation

// MockTextClient
// ==============
// In-memory TextClient with an operation log, so tests can assert both the
// final document text and the ORDER of insert/marked-text calls (IMK clients
// are sensitive to commit-before-preedit ordering).

final class MockTextClient: TextClient {

    enum Op: Equatable {
        case insert(String)
        case marked(String)
    }

    private(set) var ops: [Op] = []

    /// Committed document text (inserted text only; marked text is not part
    /// of the document until committed).
    var text: String {
        ops.compactMap { if case .insert(let s) = $0 { return s } else { return nil } }.joined()
    }

    /// The most recent marked text ("" after clearing).
    var markedText: String {
        for op in ops.reversed() {
            if case .marked(let s) = op { return s }
        }
        return ""
    }

    func insertText(_ text: String) {
        ops.append(.insert(text))
    }

    func setMarkedText(_ text: String) {
        ops.append(.marked(text))
    }
}
