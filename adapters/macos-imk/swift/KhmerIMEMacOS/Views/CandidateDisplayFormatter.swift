struct CandidateDisplayFormatter {
    static let derivedMarker = "≈"

    static func displayText(for entry: MacosCandidateDisplayEntry) -> String {
        let hintText = entry.romanHints.isEmpty
            ? derivedMarker
            : entry.romanHints.joined(separator: ", ")
        return "\(entry.output)  \(hintText)"
    }

    static func displayEntries(
        candidates: [String],
        metadata: [MacosCandidateDisplayEntry]
    ) -> [MacosCandidateDisplayEntry] {
        candidates.enumerated().map { index, candidate in
            guard index < metadata.count else {
                return MacosCandidateDisplayEntry(
                    output: candidate,
                    recommended: false,
                    romanHints: []
                )
            }
            let entry = metadata[index]
            if entry.output == candidate { return entry }
            return MacosCandidateDisplayEntry(
                output: candidate,
                recommended: entry.recommended,
                romanHints: entry.romanHints
            )
        }
    }
}
