import Foundation
#if canImport(AppKit)
import AppKit
#endif

struct CandidateDisplayFormatter {
    static let derivedMarker = "≈"
    // Model-assisted marker (ADR-0016 / ADR-0019). Prefixes a model-rescued candidate; its colour
    // says whether the word is Lexicon-verified.
    static let modelMarker = "✦"

    static func displayText(for entry: MacosCandidateDisplayEntry) -> String {
        let hintText = entry.romanHints.isEmpty
            ? derivedMarker
            : entry.romanHints.joined(separator: ", ")
        let prefix = entry.fromModel ? "\(modelMarker) " : ""
        return "\(prefix)\(entry.output)  \(hintText)"
    }

    #if canImport(AppKit)
    /// Attributed form for a candidate row: the ✦ prefix is coloured — white (default label) when
    /// the model word is Lexicon-verified, red when unverified (out-of-Lexicon; still selectable,
    /// per ADR-0016, since it may be a valid name/loanword). Non-model rows return plain text.
    static func attributedDisplayText(for entry: MacosCandidateDisplayEntry) -> NSAttributedString {
        let full = displayText(for: entry)
        let attr = NSMutableAttributedString(
            string: full,
            attributes: [.foregroundColor: NSColor.labelColor]
        )
        if entry.fromModel && !entry.lexiconVerified {
            // Colour just the leading ✦ red; the Khmer text stays normal.
            attr.addAttribute(.foregroundColor, value: NSColor.systemRed,
                              range: NSRange(location: 0, length: 1))
        }
        return attr
    }
    #endif

    static func displayEntries(
        candidates: [String],
        metadata: [MacosCandidateDisplayEntry]
    ) -> [MacosCandidateDisplayEntry] {
        candidates.enumerated().map { index, candidate in
            guard index < metadata.count else {
                return MacosCandidateDisplayEntry(
                    output: candidate,
                    recommended: false,
                    romanHints: [],
                    fromModel: false,
                    lexiconVerified: true
                )
            }
            let entry = metadata[index]
            if entry.output == candidate { return entry }
            return MacosCandidateDisplayEntry(
                output: candidate,
                recommended: entry.recommended,
                romanHints: entry.romanHints,
                fromModel: entry.fromModel,
                lexiconVerified: entry.lexiconVerified
            )
        }
    }
}
