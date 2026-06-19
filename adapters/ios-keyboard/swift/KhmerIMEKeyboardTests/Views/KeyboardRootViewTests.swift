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

        fixture.rootView.render(state, romanHint: "k", keyboardState: .qwerty)

        XCTAssertEqual(fixture.stripView.renderedRomanHint, "k")
        XCTAssertEqual(fixture.stripView.renderedState, state)
        XCTAssertEqual(fixture.candidateRowView.renderedState, state)
        XCTAssertEqual(fixture.candidateRowView.renderedPresentation, .composition)
    }

    func test_renderInCharPickUsesCharPickCandidatePresentation() {
        let fixture = makeRootView()
        let state = makeRenderState(candidates: ["ក", "ខ"])

        fixture.rootView.render(state, romanHint: "", keyboardState: .charPick)

        XCTAssertEqual(fixture.candidateRowView.renderedState, state)
        XCTAssertEqual(fixture.candidateRowView.renderedPresentation, .charPick)
    }

    func test_clearStrip_alsoClearsCandidateRow() {
        let fixture = makeRootView()
        fixture.rootView.clearStrip()

        XCTAssertEqual(fixture.stripView.clearCount, 1)
        XCTAssertEqual(fixture.candidateRowView.clearCount, 1,
            "candidate row must be cleared whenever the strip is cleared so stale candidates don't linger after a commit")
    }

    // MARK: - chrome collapse / expand

    func test_setChromeRowsNone_collapsesStripAndCandidateRowToZero() {
        let fixture = makeRootView()

        fixture.rootView.setChromeRows(.none)

        XCTAssertEqual(fixture.rootView.stripHeightConstraint.constant, 0,
            "collapsed chrome must drop the strip height to zero")
        XCTAssertEqual(fixture.rootView.candidateRowHeightConstraint.constant, 0,
            "collapsed chrome must drop the candidate row height to zero")
    }

    func test_setChromeRowsStripAndCandidate_restoresReservedRowHeights() {
        let fixture = makeRootView()
        fixture.rootView.setChromeRows(.none)

        fixture.rootView.setChromeRows(.stripAndCandidate)

        XCTAssertEqual(fixture.rootView.stripHeightConstraint.constant, 44,
            "expanded chrome must restore the strip to its reserved height")
        XCTAssertEqual(fixture.rootView.candidateRowHeightConstraint.constant, 44,
            "expanded chrome must restore the candidate row to its reserved height")
    }

    func test_setChromeRowsCandidateOnly_keepsStripCollapsedAndShowsCandidateRow() {
        let fixture = makeRootView()

        fixture.rootView.setChromeRows(.candidateOnly)

        XCTAssertEqual(fixture.rootView.stripHeightConstraint.constant, 0,
            "CharPick candidates should not reserve the roman strip row")
        XCTAssertEqual(fixture.rootView.candidateRowHeightConstraint.constant, 44,
            "CharPick candidates should reserve only the candidate row")
    }

    func test_keyLayerHeightIsFixedAndUnaffectedByChromeCollapse() {
        let fixture = makeRootView()
        let metrics = KeyboardLayoutMetrics(device: .phone)
        fixture.rootView.frame = CGRect(x: 0, y: 0, width: 320, height: metrics.baseKeyboardHeight)

        fixture.rootView.setChromeRows(.stripAndCandidate)
        fixture.rootView.layoutIfNeeded()
        XCTAssertEqual(fixture.qwertyView.bounds.height, metrics.idleKeyboardHeight, accuracy: 0.5,
            "key area must equal the fixed idle height while composing")

        fixture.rootView.setChromeRows(.none)
        fixture.rootView.layoutIfNeeded()
        XCTAssertEqual(fixture.qwertyView.bounds.height, metrics.idleKeyboardHeight, accuracy: 0.5,
            "key area height must not change when the chrome collapses — the keys never resize or move")
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
    var renderedPresentation: CandidateRowPresentation?
    var clearCount = 0

    func render(_ state: IosRenderState, presentation: CandidateRowPresentation) {
        renderedState = state
        renderedPresentation = presentation
    }

    func clear() {
        clearCount += 1
    }
}
