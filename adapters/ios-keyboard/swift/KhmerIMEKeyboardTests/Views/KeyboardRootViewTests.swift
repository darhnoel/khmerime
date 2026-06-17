import XCTest
@testable import KhmerIMEKeyboard

final class KeyboardRootViewTests: XCTestCase {

    func test_initialStateShowsQwertyLayerOnly() {
        let fixture = makeRootView()

        XCTAssertFalse(fixture.qwertyView.isHidden)
        XCTAssertTrue(fixture.numericView.isHidden)
        XCTAssertTrue(fixture.symbolsView.isHidden)
        XCTAssertFalse(fixture.candidateRowView.isHidden)
    }

    func test_applyCharPickStateKeepsQwertyVisible() {
        let fixture = makeRootView()

        fixture.rootView.apply(.charPick)

        XCTAssertFalse(fixture.qwertyView.isHidden)
        XCTAssertTrue(fixture.numericView.isHidden)
        XCTAssertTrue(fixture.symbolsView.isHidden)
    }

    func test_applyNumericStateShowsNumericLayerOnly() {
        let fixture = makeRootView()

        fixture.rootView.apply(.numeric)

        XCTAssertTrue(fixture.qwertyView.isHidden)
        XCTAssertFalse(fixture.numericView.isHidden)
        XCTAssertTrue(fixture.symbolsView.isHidden)
    }

    func test_applyCharPickStateShowsCandidateRow() {
        let fixture = makeRootView()

        fixture.rootView.apply(.charPick)

        XCTAssertFalse(fixture.candidateRowView.isHidden)
    }

    func test_renderUpdatesBothStripAndCandidateRow() {
        let fixture = makeRootView()
        let state = makeRenderState(candidates: ["ក", "ខ"])

        fixture.rootView.render(state, romanHint: "k")

        XCTAssertEqual(fixture.stripView.renderedRomanHint, "k")
        XCTAssertEqual(fixture.stripView.renderedState, state)
        XCTAssertEqual(fixture.candidateRowView.renderedState, state)
    }

    func test_clearStrip_alsoClearsCandidateRow() {
        let fixture = makeRootView()
        fixture.rootView.clearStrip()

        XCTAssertEqual(fixture.stripView.clearCount, 1)
        XCTAssertEqual(fixture.candidateRowView.clearCount, 1,
            "candidate row must be cleared whenever the strip is cleared so stale candidates don't linger after a commit")
    }

    private func makeRootView() -> (
        rootView: KeyboardRootView,
        stripView: SpyStripView,
        qwertyView: UIView,
        numericView: UIView,
        symbolsView: UIView,
        candidateRowView: SpyCandidateRowView
    ) {
        let stripView = SpyStripView()
        let candidateRowView = SpyCandidateRowView()
        let qwertyView = UIView()
        let numericView = UIView()
        let symbolsView = UIView()

        let rootView = KeyboardRootView(
            metrics: KeyboardLayoutMetrics(device: .phone),
            stripView: stripView,
            qwertyView: qwertyView,
            numericView: numericView,
            symbolsView: symbolsView,
            candidateRowView: candidateRowView
        )

        return (rootView, stripView, qwertyView, numericView, symbolsView, candidateRowView)
    }

    private func makeRenderState(candidates: [String]) -> IosRenderState {
        IosRenderState(
            candidates: candidates,
            selectedIndex: nil,
            preedit: "",
            segments: [],
            focusedSegmentIndex: nil,
            commitText: nil,
            segmentEditActive: false,
            segmentEditIndex: nil
        )
    }
}

private final class SpyStripView: UIView, KeyboardStripDisplaying {
    var renderedState: IosRenderState?
    var renderedRomanHint: String?
    var clearCount = 0

    func render(_ state: IosRenderState, romanBuffer: String) {
        renderedState = state
        renderedRomanHint = romanBuffer
    }

    func clear() {
        clearCount += 1
    }
}

private final class SpyCandidateRowView: UIView, KeyboardCandidateRowDisplaying {
    var renderedState: IosRenderState?
    var clearCount = 0

    func render(_ state: IosRenderState) {
        renderedState = state
    }

    func clear() {
        clearCount += 1
    }
}
