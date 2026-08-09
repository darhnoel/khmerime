import XCTest
import UIKit
@testable import KhmerIMEKeyboard

final class CandidateRowViewTests: XCTestCase {

    func test_showQuickAccess_displaysAppleShapedMarkButSelectsRawUnicode() {
        let row = CandidateRowView()
        row.frame = CGRect(x: 0, y: 0, width: 390, height: 44)
        var selected: String?

        row.showQuickAccess(QuickAccessSpec.marks) { selected = $0.commitText }
        row.layoutIfNeeded()

        XCTAssertEqual(visibleLabelTexts(in: row), QuickAccessSpec.marks.map(\.displayText))
        let combiningLabel = visibleLabels(in: row)[9]
        row.handleTap(at: combiningLabel.frame.center, in: combiningLabel.superview!)
        XCTAssertEqual(selected, "៊", "Apple's placeholder circle is display-only")
    }

    func test_quickAccessAndCandidatesUseReadableUnboxedLabelSpacing() {
        let row = CandidateRowView()

        row.showQuickAccess(QuickAccessSpec.marks) { _ in }
        XCTAssertEqual(horizontalStack(in: row).spacing, 13)

        row.render(makeState(candidates: ["ខ្ញុំ", "ញុំ"], selectedIndex: 0))
        XCTAssertEqual(horizontalStack(in: row).spacing, 13)
    }

    func test_quickAccessTouch_usesTextKeyPressedAnimation() {
        let row = CandidateRowView()
        row.showQuickAccess(QuickAccessSpec.marks) { _ in }
        let label = visibleLabels(in: row)[0]
        UIView.setAnimationsEnabled(false)
        defer { UIView.setAnimationsEnabled(true) }

        label.touchesBegan(Set(), with: nil)
        XCTAssertEqual(label.transform.a, 0.92, accuracy: 0.01)
        label.touchesEnded(Set(), with: nil)
        XCTAssertEqual(label.transform.a, 1, accuracy: 0.01)
    }

    func test_render_showsOneLabelPerCandidateInOrder() {
        let row = CandidateRowView()

        row.render(makeState(candidates: ["ខ្ញុំ", "ញុំ", "ណុំ"], selectedIndex: 0))

        XCTAssertEqual(visibleLabelTexts(in: row), ["ខ្ញុំ", "ញុំ", "ណុំ"])
    }

    func test_render_prefixesCoengCandidateWithDottedCircleForVisibility() {
        let row = CandidateRowView()

        row.render(makeState(candidates: ["្ក"], selectedIndex: 0))

        XCTAssertEqual(visibleLabelTexts(in: row), ["◌្ក"],
            "candidate row should make leading coeng signs visible without changing the inserted candidate")
    }

    func test_render_highlightsSelectedIndexOnly() {
        let row = CandidateRowView()

        row.render(makeState(candidates: ["ខ្ញុំ", "ញុំ", "ណុំ"], selectedIndex: 1))

        let labels = visibleLabels(in: row)
        XCTAssertEqual(labels.map { $0.textColor }, [.secondaryLabel, .label, .secondaryLabel],
            "only the selected candidate's label should use the highlighted (.label) color")
    }

    func test_render_charPickPresentationUsesUniformRegularLabels() {
        let row = CandidateRowView()

        row.render(makeState(candidates: ["ក", "្ក", "ខ"], selectedIndex: 0), presentation: .charPick)

        let labels = visibleLabels(in: row)
        let regularFontName = UIFont.systemFont(ofSize: 18, weight: .regular).fontName
        XCTAssertEqual(labels.map { $0.textColor }, [.label, .label, .label],
            "CharPick candidates are equally tappable and should not look disabled or lower confidence")
        XCTAssertEqual(labels.map { $0.font.fontName }, [regularFontName, regularFontName, regularFontName],
            "CharPick candidates should not imply a default candidate with semibold text")
    }

    func test_render_reservesVerticalGlyphClearanceForCoengCandidates() {
        let row = CandidateRowView()
        row.frame = CGRect(x: 0, y: 0, width: 300, height: 44)

        row.render(makeState(candidates: ["្ង"], selectedIndex: 0), presentation: .charPick)
        row.layoutIfNeeded()

        let label = visibleLabels(in: row)[0]
        XCTAssertGreaterThanOrEqual(label.bounds.height, label.font.lineHeight + 12,
            "Khmer labels need clearance beyond the nominal font line box for below-base marks")
        XCTAssertGreaterThanOrEqual(label.frame.minY, row.bounds.minY)
        XCTAssertLessThanOrEqual(label.frame.maxY, row.bounds.maxY)
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

    func test_render_fewCandidates_centersContentSymmetrically() {
        let row = CandidateRowView()
        row.frame = CGRect(x: 0, y: 0, width: 300, height: 44)
        row.render(makeState(candidates: ["ក"], selectedIndex: 0))
        row.layoutIfNeeded()

        let scroll = scrollView(in: row)
        XCTAssertGreaterThan(scroll.contentInset.left, 8,
            "a single candidate must be centered, not pinned to the left edge")
        XCTAssertEqual(scroll.contentInset.left, scroll.contentInset.right, accuracy: 0.5,
            "the centering inset must be symmetric so the chips sit in the middle")
    }

    func test_render_manyCandidates_usesEdgeInsetForLeftAlignedScroll() {
        let row = CandidateRowView()
        row.frame = CGRect(x: 0, y: 0, width: 300, height: 44)
        let many = (0..<20).map { "ខ្ញុំ\($0)" }
        row.render(makeState(candidates: many, selectedIndex: 0))
        row.layoutIfNeeded()

        let scroll = scrollView(in: row)
        XCTAssertEqual(scroll.contentInset.left, 8, accuracy: 0.5,
            "candidates that overflow the row must use the normal edge inset (left-aligned + scrollable)")
    }

    private func scrollView(in view: UIView) -> UIScrollView {
        for sv in view.subviews {
            if let scroll = sv as? UIScrollView { return scroll }
        }
        fatalError("CandidateRowView must contain a UIScrollView")
    }

    private func horizontalStack(in view: UIView) -> UIStackView {
        guard let stack = findHorizontalStack(in: view) else {
            fatalError("CandidateRowView must contain a horizontal UIStackView")
        }
        return stack
    }

    private func findHorizontalStack(in view: UIView) -> UIStackView? {
        for subview in view.subviews {
            if let stack = subview as? UIStackView, stack.axis == .horizontal { return stack }
            if let stack = findHorizontalStack(in: subview) { return stack }
        }
        return nil
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
            segmentEditIndex: nil,
            phraseCandidates: [],
            selectedPhraseIndex: 0
        )
    }
}

private extension CGRect {
    var center: CGPoint { CGPoint(x: midX, y: midY) }
}
