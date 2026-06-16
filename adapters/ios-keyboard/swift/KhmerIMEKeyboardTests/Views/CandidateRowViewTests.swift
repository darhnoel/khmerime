import XCTest
import UIKit
@testable import KhmerIMEKeyboard

final class CandidateRowViewTests: XCTestCase {

    func test_render_showsOneLabelPerCandidateInOrder() {
        let row = CandidateRowView()

        row.render(makeState(candidates: ["ខ្ញុំ", "ញុំ", "ណុំ"], selectedIndex: 0))

        XCTAssertEqual(visibleLabelTexts(in: row), ["ខ្ញុំ", "ញុំ", "ណុំ"])
    }

    func test_render_highlightsSelectedIndexOnly() {
        let row = CandidateRowView()

        row.render(makeState(candidates: ["ខ្ញុំ", "ញុំ", "ណុំ"], selectedIndex: 1))

        let labels = visibleLabels(in: row)
        XCTAssertEqual(labels.map { $0.textColor }, [.secondaryLabel, .label, .secondaryLabel],
            "only the selected candidate's label should use the highlighted (.label) color")
    }

    func test_tapAtPoint_onACandidateLabel_invokesOnCandidateSelected() {
        let row = CandidateRowView()
        row.frame = CGRect(x: 0, y: 0, width: 300, height: 44)
        row.render(makeState(candidates: ["ខ្ញុំ", "ញុំ", "ណុំ"], selectedIndex: 0))
        row.layoutIfNeeded()

        var selected: Int?
        row.onCandidateSelected = { selected = $0 }

        let label = visibleLabels(in: row)[1]
        row.handleTap(at: label.frame.center, in: label.superview!)

        XCTAssertEqual(selected, 1, "tapping the second candidate's label must report index 1")
    }

    func test_tapAtPoint_onEmptyRowSpace_doesNotInvokeOnCandidateSelected() {
        let row = CandidateRowView()
        row.frame = CGRect(x: 0, y: 0, width: 300, height: 44)
        row.render(makeState(candidates: ["ខ្ញុំ"], selectedIndex: 0))
        row.layoutIfNeeded()

        var selected: Int?
        row.onCandidateSelected = { selected = $0 }

        row.handleTap(at: CGPoint(x: 290, y: 22), in: row)

        XCTAssertNil(selected, "tapping empty row space (past the last chip) must not select anything")
    }

    func test_clear_hidesAllPreviouslyShownLabels() {
        let row = CandidateRowView()
        row.render(makeState(candidates: ["ខ្ញុំ", "ញុំ"], selectedIndex: 0))

        row.clear()

        XCTAssertEqual(visibleLabelTexts(in: row), [],
            "clear() must hide every candidate label that was previously shown")
    }

    private func visibleLabels(in view: UIView) -> [UILabel] {
        var result: [UILabel] = []
        for sv in view.subviews {
            if let lbl = sv as? UILabel, !lbl.isHidden { result.append(lbl) }
            result += visibleLabels(in: sv)
        }
        return result
    }

    private func visibleLabelTexts(in view: UIView) -> [String?] {
        var result: [String?] = []
        for sv in view.subviews {
            if let lbl = sv as? UILabel, !lbl.isHidden { result.append(lbl.text) }
            result += visibleLabelTexts(in: sv)
        }
        return result
    }

    private func makeState(candidates: [String], selectedIndex: Int?) -> IosRenderState {
        IosRenderState(
            candidates: candidates,
            selectedIndex: selectedIndex.map { UInt64($0) },
            preedit: "",
            segments: [],
            focusedSegmentIndex: nil,
            commitText: nil,
            segmentEditActive: false,
            segmentEditIndex: nil
        )
    }
}

private extension CGRect {
    var center: CGPoint { CGPoint(x: midX, y: midY) }
}
