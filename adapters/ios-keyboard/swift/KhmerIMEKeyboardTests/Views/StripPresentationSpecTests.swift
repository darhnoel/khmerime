import XCTest
@testable import KhmerIMEKeyboard

final class StripPresentationSpecTests: XCTestCase {

    // MARK: - romanRowText — no segments

    func test_romanRowText_noSegments_returnsRomanBuffer() {
        let state = makeState(segments: [])
        XCTAssertEqual(StripPresentationSpec.romanRowText(state: state, romanBuffer: "nhom"), "nhom")
    }

    func test_romanRowText_withSegments_joinsInputsWithDot() {
        let state = makeState(segments: [
            seg(input: "nhom", output: "ខ្ញុំ", focused: false),
            seg(input: "ttov", output: "ទៅ", focused: false),
        ])
        XCTAssertEqual(
            StripPresentationSpec.romanRowText(state: state, romanBuffer: ""),
            "nhom · ttov"
        )
    }

    func test_romanRowText_inEditMode_wrapsEditedSegmentInBrackets() {
        let state = makeState(
            segments: [
                seg(input: "nhom", output: "ខ្ញុំ", focused: false),
                seg(input: "ttov", output: "ទៅ", focused: true),
            ],
            segmentEditActive: true,
            segmentEditIndex: 1
        )
        XCTAssertEqual(
            StripPresentationSpec.romanRowText(state: state, romanBuffer: ""),
            "✏ nhom · [ttov]"
        )
    }

    // MARK: - segmentKhmerTexts

    func test_segmentKhmerTexts_noSegments_returnsEmpty() {
        let state = makeState(segments: [])
        XCTAssertEqual(StripPresentationSpec.segmentKhmerTexts(state: state), [])
    }

    func test_segmentKhmerTexts_returnsOutputsInOrder() {
        let state = makeState(segments: [
            seg(input: "nhom", output: "ខ្ញុំ", focused: false),
            seg(input: "ttov", output: "ទៅ", focused: true),
        ])
        XCTAssertEqual(StripPresentationSpec.segmentKhmerTexts(state: state), ["ខ្ញុំ", "ទៅ"])
    }

    // MARK: - focusedSegmentIndex

    func test_focusedSegmentIndex_noSegments_returnsNil() {
        let state = makeState(segments: [])
        XCTAssertNil(StripPresentationSpec.focusedSegmentIndex(state: state))
    }

    func test_focusedSegmentIndex_returnsIndexOfFocusedSegment() {
        let state = makeState(segments: [
            seg(input: "nhom", output: "ខ្ញុំ", focused: false),
            seg(input: "ttov", output: "ទៅ", focused: true),
            seg(input: "salarien", output: "សាលារៀន", focused: false),
        ])
        XCTAssertEqual(StripPresentationSpec.focusedSegmentIndex(state: state), 1)
    }

    func test_focusedSegmentIndex_noFocusedSegment_returnsNil() {
        let state = makeState(segments: [
            seg(input: "nhom", output: "ខ្ញុំ", focused: false),
            seg(input: "ttov", output: "ទៅ", focused: false),
        ])
        XCTAssertNil(StripPresentationSpec.focusedSegmentIndex(state: state))
    }

    // MARK: - Helpers

    private func makeState(
        segments: [IosSegmentEntry],
        segmentEditActive: Bool = false,
        segmentEditIndex: Int? = nil
    ) -> IosRenderState {
        IosRenderState(
            candidates: [],
            selectedIndex: nil,
            preedit: "",
            segments: segments,
            focusedSegmentIndex: segments.firstIndex(where: { $0.focused }).map { UInt64($0) },
            commitText: nil,
            segmentEditActive: segmentEditActive,
            segmentEditIndex: segmentEditIndex.map { UInt64($0) }
        )
    }

    private func seg(input: String, output: String, focused: Bool) -> IosSegmentEntry {
        IosSegmentEntry(output: output, input: input, focused: focused)
    }
}
