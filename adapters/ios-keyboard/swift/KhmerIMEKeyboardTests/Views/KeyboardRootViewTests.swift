import XCTest
@testable import KhmerIMEKeyboard

final class KeyboardRootViewTests: XCTestCase {

    func test_initialStateShowsQwertyLayerOnly() {
        let fixture = makeRootView()

        XCTAssertFalse(fixture.qwertyView.isHidden)
        XCTAssertTrue(fixture.numericView.isHidden)
        XCTAssertTrue(fixture.symbolsView.isHidden)
        XCTAssertFalse(fixture.candidateRowView.isHidden)
        XCTAssertTrue(fixture.panelView.isHidden)
        XCTAssertTrue(fixture.panelBottomRow.isHidden)
    }

    func test_applyCharPickStateShowsPanelAndBottomRow() {
        let fixture = makeRootView()

        fixture.rootView.apply(.charPick)

        XCTAssertTrue(fixture.qwertyView.isHidden)
        XCTAssertTrue(fixture.numericView.isHidden)
        XCTAssertTrue(fixture.symbolsView.isHidden)
        XCTAssertFalse(fixture.panelView.isHidden)
        XCTAssertFalse(fixture.panelBottomRow.isHidden)
    }

    func test_applyNumericStateShowsNumericLayerOnly() {
        let fixture = makeRootView()

        fixture.rootView.apply(.numeric)

        XCTAssertTrue(fixture.qwertyView.isHidden)
        XCTAssertFalse(fixture.numericView.isHidden)
        XCTAssertTrue(fixture.symbolsView.isHidden)
        XCTAssertTrue(fixture.panelView.isHidden)
        XCTAssertTrue(fixture.panelBottomRow.isHidden)
    }

    func test_applyCharPickStateHidesCandidateRow() {
        let fixture = makeRootView()

        fixture.rootView.apply(.charPick)

        XCTAssertTrue(fixture.candidateRowView.isHidden)
    }

    func test_renderCharPickStateUpdatesStripAndOnlyPanelCandidates() {
        let fixture = makeRootView()
        let state = makeRenderState(candidates: ["ក", "ខ"])

        fixture.rootView.render(state, romanHint: "k", keyboardState: .charPick)

        XCTAssertEqual(fixture.stripView.renderedRomanHint, "k")
        XCTAssertEqual(fixture.stripView.renderedState, state)
        XCTAssertNil(fixture.panelView.renderedState)
        XCTAssertEqual(fixture.panelView.renderedCharPickCandidates, ["ក", "ខ"])
    }

    func test_renderCharPickStateClearsCandidateRow() {
        let fixture = makeRootView()
        let qwertyState = makeRenderState(candidates: ["ក"])
        let charPickState = makeRenderState(candidates: ["ខ"])

        fixture.rootView.render(qwertyState, romanHint: "k", keyboardState: .qwerty)
        fixture.rootView.render(charPickState, romanHint: "kh", keyboardState: .charPick)

        XCTAssertEqual(fixture.candidateRowView.clearCount, 1)
    }

    func test_renderQwertyStateUpdatesStripAndCandidateRow() {
        let fixture = makeRootView()
        let state = makeRenderState(candidates: ["ក", "ខ"])

        fixture.rootView.render(state, romanHint: "k", keyboardState: .qwerty)

        XCTAssertEqual(fixture.stripView.renderedRomanHint, "k")
        XCTAssertEqual(fixture.stripView.renderedState, state)
        XCTAssertEqual(fixture.candidateRowView.renderedState, state)
    }

    func test_clearStripAndRenderCharPickAlphabetForwardToContainedViews() {
        let fixture = makeRootView()

        fixture.rootView.clearStrip()
        fixture.rootView.renderCharPickAlphabet()

        XCTAssertEqual(fixture.stripView.clearCount, 1)
        XCTAssertEqual(fixture.panelView.alphabetRenderCount, 1)
    }

    private func makeRootView() -> (
        rootView: KeyboardRootView,
        stripView: SpyStripView,
        qwertyView: UIView,
        numericView: UIView,
        symbolsView: UIView,
        candidateRowView: SpyCandidateRowView,
        panelView: SpyPanelView,
        panelBottomRow: UIView
    ) {
        let stripView = SpyStripView()
        let candidateRowView = SpyCandidateRowView()
        let panelView = SpyPanelView()
        let qwertyView = UIView()
        let numericView = UIView()
        let symbolsView = UIView()
        let panelBottomRow = UIView()

        let rootView = KeyboardRootView(
            metrics: KeyboardLayoutMetrics(device: .phone),
            stripView: stripView,
            qwertyView: qwertyView,
            numericView: numericView,
            symbolsView: symbolsView,
            candidateRowView: candidateRowView,
            panelView: panelView,
            panelBottomRow: panelBottomRow,
            panelBottomAnchorGuide: panelView.bottomAnchorGuide
        )

        return (rootView, stripView, qwertyView, numericView, symbolsView, candidateRowView, panelView, panelBottomRow)
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

private final class SpyPanelView: UIView, KeyboardPanelDisplaying {
    let bottomAnchorGuide = UILayoutGuide()
    var renderedState: IosRenderState?
    var renderedCharPickCandidates: [String] = []
    var alphabetRenderCount = 0

    override init(frame: CGRect) {
        super.init(frame: frame)
        addLayoutGuide(bottomAnchorGuide)
        NSLayoutConstraint.activate([
            bottomAnchorGuide.topAnchor.constraint(equalTo: bottomAnchor),
            bottomAnchorGuide.leadingAnchor.constraint(equalTo: leadingAnchor),
            bottomAnchorGuide.trailingAnchor.constraint(equalTo: trailingAnchor),
        ])
    }

    required init?(coder: NSCoder) { fatalError("use init(frame:)") }

    func render(_ state: IosRenderState) {
        renderedState = state
    }

    func renderCharPickCandidates(_ candidates: [String]) {
        renderedCharPickCandidates = candidates
    }

    func renderCharPickAlphabet() {
        alphabetRenderCount += 1
    }
}
