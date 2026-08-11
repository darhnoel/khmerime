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
