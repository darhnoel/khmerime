import XCTest

final class CandidateDisplayFormatterTests: XCTestCase {
    func test_exactRomanHintsRenderOnOneLine() {
        let entry = MacosCandidateDisplayEntry(
            output: "ជា",
            recommended: true,
            romanHints: ["jea"],
            fromModel: false,
            lexiconVerified: true
        )

        let text = CandidateDisplayFormatter.displayText(for: entry)

        XCTAssertEqual(text, "ជា  jea")
        XCTAssertFalse(text.contains("\n"))
    }

    func test_multipleExactRomanHintsAreNotInventedOrHidden() {
        let entry = MacosCandidateDisplayEntry(
            output: "ទៅ",
            recommended: false,
            romanHints: ["tov", "to"],
            fromModel: false,
            lexiconVerified: true
        )

        XCTAssertEqual(CandidateDisplayFormatter.displayText(for: entry), "ទៅ  tov, to")
    }

    func test_missingRomanHintsUseDerivedMarker() {
        let entry = MacosCandidateDisplayEntry(
            output: "ខ្មែរ",
            recommended: false,
            romanHints: [],
            fromModel: false,
            lexiconVerified: true
        )

        XCTAssertEqual(CandidateDisplayFormatter.displayText(for: entry), "ខ្មែរ  ≈")
    }

    func test_phraseModeDropsRomanHint_khmerOnly() {
        let entry = MacosCandidateDisplayEntry(
            output: "អ្នកបន្ថែមទៀត",
            recommended: true,
            romanHints: ["neak bonthaem tiet"],
            fromModel: false,
            lexiconVerified: true
        )

        // Phrase mode carries the roman in the header row, not on every candidate (ADR-0004).
        XCTAssertEqual(CandidateDisplayFormatter.displayText(for: entry, mode: .phrase), "អ្នកបន្ថែមទៀត")
    }

    func test_segmentModeDropsRomanHintButKeepsModelMarker() {
        let entry = MacosCandidateDisplayEntry(
            output: "ទៅ",
            recommended: false,
            romanHints: ["tov"],
            fromModel: true,
            lexiconVerified: true
        )

        XCTAssertEqual(CandidateDisplayFormatter.displayText(for: entry, mode: .segment), "✦ ទៅ")
    }

    func test_missingMetadataFallsBackToDerivedCandidateEntry() {
        let entries = CandidateDisplayFormatter.displayEntries(
            candidates: ["ទៅ"],
            metadata: []
        )

        XCTAssertEqual(entries, [
            MacosCandidateDisplayEntry(output: "ទៅ", recommended: false, romanHints: [], fromModel: false, lexiconVerified: true)
        ])
    }
}
