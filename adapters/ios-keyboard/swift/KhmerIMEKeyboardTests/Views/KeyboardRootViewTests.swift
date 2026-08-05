import XCTest
@testable import KhmerIMEKeyboard

final class KeyboardRootViewTests: XCTestCase {

    func test_keyboardRootEnablesSystemInputClicks() {
        let rootView = makeRootView().rootView

        XCTAssertTrue(rootView.enableInputClicksWhenVisible)
    }

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

    func test_renderInCharPickClearsStripInsteadOfRenderingFirstCandidate() {
        let fixture = makeRootView()
        let state = makeRenderState(candidates: ["ង", "្ង", "ញ"])

        fixture.rootView.render(state, romanHint: "", keyboardState: .charPick)

        XCTAssertNil(fixture.stripView.renderedState,
            "CharPick candidates belong only in the Candidate List, never in the collapsed Strip")
        XCTAssertEqual(fixture.stripView.clearCount, 1,
            "CharPick rendering must clear any previously rendered Strip preview")
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

    func test_setChromeRowsStripOnly_collapsesCandidateRowButKeepsStrip() {
        let fixture = makeRootView()

        fixture.rootView.setChromeRows(.stripOnly)

        XCTAssertEqual(fixture.rootView.stripHeightConstraint.constant, 56,
            "strip height should remain reserved for the selected phrase preview")
        XCTAssertEqual(fixture.rootView.candidateRowHeightConstraint.constant, 0,
            "candidate row height should collapse when there are no phrase alternatives")
    }

    func test_keyPreviewReanchorsWhenChromeExpansionResizesTheKeyboard() {
        // The first keystroke of a composition expands the chrome: the host grows,
        // the bottom-anchored keys shift down within rootView, and a statically
        // framed popup would ride up over the strip. The popup must re-anchor to
        // its source key on layout.
        let fixture = makeRootView()
        let metrics = KeyboardLayoutMetrics(device: .phone)
        fixture.rootView.frame = CGRect(x: 0, y: 0, width: 390, height: metrics.idleKeyboardHeight)
        fixture.rootView.layoutIfNeeded()
        let key = UIView(frame: CGRect(x: 178, y: 60, width: 34, height: 44))
        fixture.qwertyView.addSubview(key)

        fixture.rootView.showKeyPreview(label: "J", from: key)
        let popup = fixture.rootView.subviews.compactMap { $0 as? KeyPreviewPopupView }.first
        let frameWhileCollapsed = popup?.frame ?? .zero

        // Composing: host grows by the strip + candidate row.
        fixture.rootView.frame = CGRect(x: 0, y: 0, width: 390, height: metrics.baseKeyboardHeight)
        fixture.rootView.layoutIfNeeded()

        let expected = KeyPreviewPopupView.frame(
            sourceFrame: key.convert(key.bounds, to: fixture.rootView),
            in: fixture.rootView.bounds
        )
        guard let popup else { return XCTFail("expected a key preview popup") }
        XCTAssertEqual(popup.frame.minX, expected.minX, accuracy: 0.001)
        XCTAssertEqual(popup.frame.minY, expected.minY, accuracy: 0.001)
        XCTAssertEqual(popup.frame.width, expected.width, accuracy: 0.001)
        XCTAssertEqual(popup.frame.height, expected.height, accuracy: 0.001,
            "popup must re-anchor to its key after the keyboard resizes")
        XCTAssertNotEqual(popup.frame, frameWhileCollapsed,
            "keys moved down with the expanded chrome, so a correctly anchored popup cannot keep its old frame")
    }

    func test_setChromeRowsStripAndCandidate_restoresReservedRowHeights() {
        let fixture = makeRootView()
        fixture.rootView.setChromeRows(.none)

        fixture.rootView.setChromeRows(.stripAndCandidate)

        XCTAssertEqual(fixture.rootView.stripHeightConstraint.constant, 56,
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

    // MARK: - key preview popup

    func test_showKeyPreview_addsOneTopmostOverlayAboveTheSourceKey() {
        let fixture = makeRootView()
        fixture.rootView.frame = CGRect(x: 0, y: 0, width: 320, height: 260)
        let key = UIView(frame: CGRect(x: 100, y: 180, width: 32, height: 44))
        fixture.rootView.addSubview(key)

        fixture.rootView.showKeyPreview(label: "A", from: key)

        let popups = fixture.rootView.subviews.compactMap { $0 as? KeyPreviewPopupView }
        XCTAssertEqual(popups.count, 1)
        XCTAssertEqual(popups[0].previewLabel, "A")
        XCTAssertTrue(fixture.rootView.subviews.last === popups[0],
            "key preview popup should render above chrome and key layers")
        XCTAssertLessThan(popups[0].frame.minY, key.frame.minY,
            "key preview popup should sit above the source key")
    }

    func test_showKeyPreview_replacesExistingOverlay() {
        let fixture = makeRootView()
        fixture.rootView.frame = CGRect(x: 0, y: 0, width: 320, height: 260)
        let firstKey = UIView(frame: CGRect(x: 40, y: 180, width: 32, height: 44))
        let secondKey = UIView(frame: CGRect(x: 180, y: 180, width: 32, height: 44))
        fixture.rootView.addSubview(firstKey)
        fixture.rootView.addSubview(secondKey)

        fixture.rootView.showKeyPreview(label: "A", from: firstKey)
        fixture.rootView.showKeyPreview(label: "B", from: secondKey)

        let popups = fixture.rootView.subviews.compactMap { $0 as? KeyPreviewPopupView }
        XCTAssertEqual(popups.count, 1)
        XCTAssertEqual(popups.first?.previewLabel, "B")
    }

    func test_hideKeyPreview_removesOverlay() {
        let fixture = makeRootView()
        fixture.rootView.frame = CGRect(x: 0, y: 0, width: 320, height: 260)
        let key = UIView(frame: CGRect(x: 100, y: 180, width: 32, height: 44))
        fixture.rootView.addSubview(key)

        fixture.rootView.showKeyPreview(label: "A", from: key)
        fixture.rootView.hideKeyPreview()

        XCTAssertTrue(fixture.rootView.subviews.compactMap { $0 as? KeyPreviewPopupView }.isEmpty)
    }

    func test_keyPreviewFrameClampsInsideRootBoundsForEdgeKeys() {
        let fixture = makeRootView()
        fixture.rootView.frame = CGRect(x: 0, y: 0, width: 320, height: 260)
        let leftKey = UIView(frame: CGRect(x: 0, y: 180, width: 32, height: 44))
        let rightKey = UIView(frame: CGRect(x: 288, y: 180, width: 32, height: 44))
        fixture.rootView.addSubview(leftKey)
        fixture.rootView.addSubview(rightKey)

        fixture.rootView.showKeyPreview(label: "Q", from: leftKey)
        let leftPopup = fixture.rootView.subviews.compactMap { $0 as? KeyPreviewPopupView }.first!
        XCTAssertGreaterThanOrEqual(leftPopup.frame.minX, fixture.rootView.bounds.minX + KeyPreviewPopupView.edgeInset)

        fixture.rootView.showKeyPreview(label: "P", from: rightKey)
        let rightPopup = fixture.rootView.subviews.compactMap { $0 as? KeyPreviewPopupView }.first!
        XCTAssertLessThanOrEqual(rightPopup.frame.maxX, fixture.rootView.bounds.maxX - KeyPreviewPopupView.edgeInset)
    }

    func test_keyPreviewDoesNotChangeChromeOrKeyLayerHeights() {
        let fixture = makeRootView()
        fixture.rootView.frame = CGRect(x: 0, y: 0, width: 320, height: KeyboardLayoutMetrics(device: .phone).baseKeyboardHeight)
        fixture.rootView.setChromeRows(.stripAndCandidate)
        fixture.rootView.layoutIfNeeded()
        let stripHeight = fixture.rootView.stripHeightConstraint.constant
        let candidateHeight = fixture.rootView.candidateRowHeightConstraint.constant
        let qwertyHeight = fixture.qwertyView.bounds.height
        let key = UIView(frame: CGRect(x: 100, y: 180, width: 32, height: 44))
        fixture.rootView.addSubview(key)

        fixture.rootView.showKeyPreview(label: "A", from: key)
        fixture.rootView.hideKeyPreview()

        XCTAssertEqual(fixture.rootView.stripHeightConstraint.constant, stripHeight)
        XCTAssertEqual(fixture.rootView.candidateRowHeightConstraint.constant, candidateHeight)
        XCTAssertEqual(fixture.qwertyView.bounds.height, qwertyHeight)
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
            segmentEditIndex: nil,
            phraseCandidates: [],
            selectedPhraseIndex: 0
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
