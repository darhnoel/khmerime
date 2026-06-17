import XCTest
@testable import KhmerIMEKeyboard

// KeyboardInputHandlerTests
// =========================
// Integration-style tests: real Rust session, mock text proxy.
// Each test verifies observable behavior through the public interface —
// what ends up in the text field — not internal state.

final class KeyboardInputHandlerTests: XCTestCase {

    // Convenience: create a fresh handler with a clean MockTextProxy.
    // SynchronousDispatcher keeps all session work inline so tests remain
    // deterministic without XCTestExpectation.
    private func makeHandler() -> (KeyboardInputHandler, MockTextProxy) {
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: SynchronousDispatcher())
        handler.focusIn()
        return (handler, proxy)
    }

    private func type(_ word: String, into handler: KeyboardInputHandler) {
        word.forEach { handler.sendChar(String($0)) }
    }

    // MARK: - Test B: basic commit

    // Standard IME contract (like Japanese/Chinese keyboards): ⏎ with an active
    // preedit COMMITS the composition only — no newline. A second ⏎ (no preedit)
    // inserts the newline.

    func test_returnWithPreedit_commitsWithoutNewline() {
        let (handler, proxy) = makeHandler()
        type("nhom", into: handler)

        handler.returnTapped()
        handler.textDidChange()     // UIKit fires this after the insertion settles

        XCTAssertFalse(proxy.text.contains("nhom"),
            "roman chars must be deleted on commit")
        XCTAssertFalse(proxy.text.isEmpty,
            "committed text must not be empty")
        XCTAssertFalse(proxy.text.contains("\n"),
            "⏎ with active preedit must commit only — no newline; got \(proxy.text.debugDescription)")
        let nonKhmer = proxy.text.unicodeScalars.filter { !(0x1780...0x17FF).contains($0.value) }
        XCTAssertTrue(nonKhmer.isEmpty,
            "committed text must contain only Khmer characters, got: \(proxy.text.debugDescription)")
    }

    func test_returnTwice_firstCommitsSecondInsertsNewline() {
        let (handler, proxy) = makeHandler()
        type("nhom", into: handler)

        handler.returnTapped()      // commit (preedit active)
        handler.textDidChange()
        handler.returnTapped()      // newline (no preedit)

        XCTAssertTrue(proxy.text.hasSuffix("\n"),
            "second ⏎ with no preedit must insert the newline; got \(proxy.text.debugDescription)")
        XCTAssertFalse(proxy.text.hasSuffix(" \n"),
            "no space before the newline; got \(proxy.text.debugDescription)")
    }

    // MARK: - Test A: no trailing space before newline

    func test_spaceReturn_noTrailingSpaceBeforeNewline() {
        let (handler, proxy) = makeHandler()
        type("nhom", into: handler)
        handler.spaceTapped()   // commits ខ្ញុំ, inserts " ", sets trailingSpace
        handler.returnTapped()  // must delete " " before inserting "\n"
        handler.textDidChange()

        XCTAssertTrue(proxy.text.hasSuffix("\n"),
            "returnTapped must insert newline")
        let beforeNewline = String(proxy.text.dropLast())
        XCTAssertFalse(beforeNewline.hasSuffix(" "),
            "trailing space must be removed before newline; got \(proxy.text.debugDescription)")
    }

    func test_returnWithNoComposition_insertsNewlineOnly() {
        let (handler, proxy) = makeHandler()

        handler.returnTapped()
        // No prior composition → pendingAutoSpaceCheck is false → "\n" inserted
        // synchronously, no textDidChange() needed.

        XCTAssertEqual(proxy.text, "\n",
            "return with no composition must insert only a newline")
    }

    func test_returnCommit_doesNotIntroduceSpace() {
        let (handler, proxy) = makeHandler()
        type("nhom", into: handler)
        handler.returnTapped()      // commit only (preedit active)
        handler.textDidChange()

        XCTAssertFalse(proxy.text.contains(" "),
            "⏎ commit must not introduce a space")
    }

    // MARK: - Space behavior

    func test_space_commitsCompositionThenInsertsSpace() {
        let (handler, proxy) = makeHandler()
        type("nhom", into: handler)
        handler.spaceTapped()

        XCTAssertFalse(proxy.text.contains("nhom"),
            "roman chars must be deleted after space")
        XCTAssertTrue(proxy.text.hasSuffix(" "),
            "space must be appended after committed word")
    }

    func test_spaceReturn_multiWord_preservesSpaceBetweenWords() {
        let (handler, proxy) = makeHandler()
        type("nhom", into: handler)
        handler.spaceTapped()
        type("ttov", into: handler)
        handler.returnTapped()      // commits ttov (preedit active)
        handler.textDidChange()
        handler.returnTapped()      // newline (no preedit)

        let text = proxy.text
        let newlineIndex = text.lastIndex(of: "\n")!
        let beforeNewline = String(text[..<newlineIndex])
        XCTAssertTrue(beforeNewline.contains(" "),
            "space between committed words must be preserved; got \(text.debugDescription)")
        XCTAssertFalse(beforeNewline.hasSuffix(" "),
            "must not have trailing space before newline; got \(text.debugDescription)")
    }

    // MARK: - iOS autocorrect auto-space

    func test_commit_removesIOSAutoSpace() {
        // Simulates real device: iOS appends " " after deleteBackward×N + insertText.
        // ⏎ with preedit commits only; textDidChange() removes the auto-space.
        // A second ⏎ then inserts a clean newline.
        let proxy = MockTextProxy()
        proxy.autoSpaceAfterInsert = true
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: SynchronousDispatcher())
        handler.focusIn()
        type("nhom", into: handler)

        handler.returnTapped()      // commit (auto-space appended by mock)
        handler.textDidChange()     // removes auto-space
        handler.returnTapped()      // newline

        XCTAssertFalse(proxy.text.hasSuffix(" \n"),
            "iOS auto-space must be removed before newline; got \(proxy.text.debugDescription)")
        XCTAssertTrue(proxy.text.hasSuffix("\n"),
            "must still end with newline")
    }

    func test_commit_removesIOSAutoSpace_onStripTap() {
        // Strip-tap calls commitComposition() directly (no returnTapped).
        // textDidChange() still handles the deferred auto-space check.
        let proxy = MockTextProxy()
        proxy.autoSpaceAfterInsert = true
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: SynchronousDispatcher())
        handler.focusIn()
        type("nhom", into: handler)

        handler.commitComposition()
        handler.textDidChange()

        XCTAssertFalse(proxy.text.hasSuffix(" "),
            "iOS auto-space must be removed after strip-tap commit; got \(proxy.text.debugDescription)")
    }

    // MARK: - Panel state machine

    func test_togglePanel_withComposition_entersCharPick() {
        // ✦ always enters CharPick now — the persistent candidate row (not the
        // panel) is the surface for browsing candidates of an active composition.
        let (handler, _) = makeHandler()
        type("nhom", into: handler)

        handler.togglePanel()

        XCTAssertEqual(handler.keyboardState, .charPick,
            "✦ must enter charPick mode regardless of composition state")
    }

    func test_togglePanel_whenInCharPick_transitionsToQwerty() {
        let (handler, _) = makeHandler()
        handler.togglePanel()   // → .charPick

        handler.togglePanel()   // → .qwerty

        XCTAssertEqual(handler.keyboardState, .qwerty,
            "second ✦ tap must dismiss charPick and return to qwerty")
    }

    func test_togglePanel_withoutComposition_transitionsToCharPick() {
        let (handler, _) = makeHandler()
        // No typing — no candidates, no composition.
        handler.togglePanel()

        XCTAssertEqual(handler.keyboardState, .charPick,
            "✦ with no composition must enter charPick mode")
    }

    func test_onTransition_firedForEveryStateChange() {
        let (handler, _) = makeHandler()
        type("nhom", into: handler)

        var transitions: [KeyboardState] = []
        handler.onTransition = { transitions.append($0) }

        handler.togglePanel()   // → .charPick
        handler.togglePanel()   // → .qwerty

        XCTAssertEqual(transitions, [.charPick, .qwerty],
            "onTransition must fire once per state change in order")
    }

    // MARK: - Tap a strip chip to edit it

    func test_chipTapped_whenComposingSingleWord_commitsInsteadOfOpeningPanel() {
        let (handler, proxy) = makeHandler()
        type("nhom", into: handler)   // single word → segments.isEmpty

        handler.chipTapped(at: 0)
        handler.textDidChange()

        XCTAssertEqual(handler.keyboardState, .qwerty,
            "tapping a single-word chip must commit, not open the panel")
        XCTAssertFalse(proxy.text.contains("nhom"),
            "roman chars must be replaced by the committed Khmer text")
    }

    func test_chipTapped_onDifferentSegment_navigatesFocusWithoutChangingState() {
        let (handler, _) = makeHandler()
        type("khnhomtov", into: handler)   // multi-word phrase, focus starts at segment 0

        handler.chipTapped(at: 1)

        XCTAssertEqual(handler.keyboardState, .qwerty,
            "tapping a different segment must navigate focus only — no panel, no state change")
        XCTAssertEqual(handler.lastState?.focusedSegmentIndex.map { Int($0) }, 1,
            "tapping segment 1 must move focus there")
    }

    func test_chipTapped_onAlreadyFocusedSegment_entersInlineEditMode() {
        let (handler, _) = makeHandler()
        type("khnhomtov", into: handler)   // multi-word phrase, focus starts at segment 0

        handler.chipTapped(at: 0)   // re-tap the already-focused segment

        XCTAssertEqual(handler.keyboardState, .qwerty,
            "2-tap-to-edit must stay in qwerty — edit mode renders inline, not via a panel")
        XCTAssertEqual(handler.lastState?.segmentEditActive, true,
            "re-tapping the focused segment must enter Segment Edit Mode")
    }

    // MARK: - Segment Edit Mode

    func test_enterSegmentEditMode_syncsProxyToEditSegment() {
        let (handler, proxy) = makeHandler()
        type("khnhomtov", into: handler)   // multi-word → segments populated

        guard let state = handler.lastState, !state.segments.isEmpty else {
            XCTFail("khnhomtov must produce segments; got state: \(String(describing: handler.lastState))")
            return
        }
        let focused = state.focusedSegmentIndex.map { Int($0) } ?? 0
        let editRoman = state.segments[focused].input

        handler.chipTapped(at: focused)   // re-tap focused segment → enters Segment Edit Mode

        XCTAssertTrue(handler.lastState?.segmentEditActive == true,
            "chipTapped on focused segment must enter Segment Edit Mode")
        XCTAssertEqual(proxy.text, editRoman,
            "entering Segment Edit Mode must replace the proxy with the edit segment's roman input")
    }

    func test_segmentEditMode_selectCandidate_exitEditModeAndPinsSegment() {
        let (handler, _) = makeHandler()
        type("khnhomtov", into: handler)
        guard let state = handler.lastState, !state.segments.isEmpty else {
            XCTFail("khnhomtov must produce segments"); return
        }
        let focused = state.focusedSegmentIndex.map { Int($0) } ?? 0
        handler.chipTapped(at: focused)   // enters Segment Edit Mode
        XCTAssertTrue(handler.lastState?.segmentEditActive == true)

        handler.selectCandidate(at: 0)   // user taps first candidate to pin the segment

        XCTAssertFalse(handler.lastState?.segmentEditActive == true,
            "selecting a candidate in Segment Edit Mode must exit edit mode and pin the segment")
        XCTAssertFalse(handler.lastState?.segments.isEmpty ?? true,
            "segments must be preserved after pinning")
    }

    func test_segmentEditMode_backspace_removesFromEditSegmentNotFullBuffer() {
        let (handler, proxy) = makeHandler()
        type("khnhomtov", into: handler)

        guard let state = handler.lastState, !state.segments.isEmpty else {
            XCTFail("khnhomtov must produce segments")
            return
        }
        let focused = state.focusedSegmentIndex.map { Int($0) } ?? 0
        let editRoman = state.segments[focused].input

        handler.chipTapped(at: focused)   // enters Segment Edit Mode; proxy = editRoman
        handler.backspaceTapped()         // should remove last char of editRoman only

        XCTAssertEqual(proxy.text, String(editRoman.dropLast()),
            "backspace in Segment Edit Mode must remove the last char of the edit segment, not the full roman buffer")
    }

    // MARK: - Layer switching

    func test_switchLayer_toNumeric() {
        let (handler, _) = makeHandler()
        handler.switchLayer(to: .numeric)
        XCTAssertEqual(handler.keyboardState, .numeric)
    }

    func test_switchLayer_toSymbols() {
        let (handler, _) = makeHandler()
        handler.switchLayer(to: .symbols)
        XCTAssertEqual(handler.keyboardState, .symbols)
    }

    func test_switchLayer_backToQwerty() {
        let (handler, _) = makeHandler()
        handler.switchLayer(to: .numeric)
        handler.switchLayer(to: .qwerty)
        XCTAssertEqual(handler.keyboardState, .qwerty)
    }

    // MARK: - CharPick mode

    // MARK: - CharPick letter tapping

    func test_charPickSelect_insertsKhmerToProxy() {
        let (handler, proxy) = makeHandler()
        handler.togglePanel()
        handler.sendChar("k")   // loads candidates incl. ក

        handler.selectCandidate(at: 0)

        XCTAssertFalse(proxy.text.isEmpty,
            "selecting a charPick candidate must insert text into the proxy")
        let isKhmer = proxy.text.unicodeScalars.allSatisfy { (0x1780...0x17FF).contains($0.value) }
        XCTAssertTrue(isKhmer,
            "inserted text must be Khmer Unicode; got \(proxy.text.debugDescription)")
    }

    func test_charPickSelect_clearsCandidateRowAndStaysInCharPick() {
        let (handler, _) = makeHandler()
        handler.togglePanel()
        handler.sendChar("k")   // loads candidates

        var stripCleared = false
        handler.onStripClear = { stripCleared = true }
        handler.selectCandidate(at: 0)

        XCTAssertTrue(stripCleared,
            "selecting a charPick candidate must clear the candidate row for the next pick")
        XCTAssertEqual(handler.keyboardState, .charPick,
            "charPick mode must persist after candidate selection")
    }

    func test_sendChar_inCharPickMode_doesNotModifyProxy() {
        let (handler, proxy) = makeHandler()
        handler.togglePanel()   // no composition → charPick
        let textBefore = proxy.text

        handler.sendChar("k")

        XCTAssertEqual(proxy.text, textBefore,
            "sendChar in charPick must not insert roman chars into the proxy")
    }

    func test_sendChar_inCharPickMode_firesOnRenderWithKhmerCandidates() {
        let (handler, _) = makeHandler()
        handler.togglePanel()   // → charPick

        var renderedCandidates: [String] = []
        handler.onRender = { state, _ in renderedCandidates = state.candidates }

        handler.sendChar("k")

        XCTAssertFalse(renderedCandidates.isEmpty,
            "sendChar in charPick must fire onRender with session candidates")
        XCTAssertTrue(renderedCandidates.contains("ក"),
            "candidates for 'k' in charPick must include ក; got \(renderedCandidates)")
    }

    func test_backspace_inCharPickMode_withCandidates_clearsCandidatesAndStaysInCharPick() {
        let (handler, proxy) = makeHandler()
        handler.togglePanel()   // → charPick
        handler.sendChar("k")   // loads candidates
        let textBefore = proxy.text

        var stripCleared = false
        handler.onStripClear = { stripCleared = true }

        handler.backspaceTapped()

        XCTAssertEqual(proxy.text, textBefore,
            "backspace in charPick with candidates must not delete from proxy")
        XCTAssertTrue(stripCleared,
            "backspace in charPick with candidates must clear the candidate row")
        XCTAssertEqual(handler.keyboardState, .charPick,
            "backspace in charPick must stay in charPick mode")
    }

    func test_backspace_inCharPickMode_withNoCandidates_deletesFromProxy() {
        let (handler, proxy) = makeHandler()
        proxy.insertText("ក")   // some committed text
        handler.togglePanel()   // → charPick, no candidates yet

        handler.backspaceTapped()

        XCTAssertEqual(proxy.text, "",
            "backspace in charPick with no candidates must delete from proxy")
    }

    // MARK: - Render callback

    func test_sendChar_firesOnRender() {
        let (handler, _) = makeHandler()
        var renderCount = 0
        handler.onRender = { _, _ in renderCount += 1 }

        type("nhom", into: handler)

        // Each char fires two renders: one optimistic (roman-hint update, immediate) and
        // one deferred (Rust-confirmed candidates). 4 chars × 2 = 8 total renders.
        XCTAssertEqual(renderCount, 8,
            "onRender fires twice per typed character — once optimistically, once after Rust")
    }

    // MARK: - External text change

    func test_textDidChange_whenProxyClearedExternally_clearsStrip() {
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: SynchronousDispatcher())
        handler.focusIn()
        type("nhom", into: handler)

        var stripCleared = false
        handler.onStripClear = { stripCleared = true }

        // Simulate the host app clearing its text field (e.g. search bar ✕ button).
        proxy.text = ""
        handler.textDidChange()

        XCTAssertTrue(stripCleared,
            "strip must clear when an external change wipes the roman buffer from the text field")
    }

    // MARK: - Backspace

    func test_backspace_removesLastTypedChar() {
        let (handler, proxy) = makeHandler()
        type("nho", into: handler)
        handler.backspaceTapped()

        // After backspace the speculative "o" should be gone from the text field.
        XCTAssertFalse(proxy.text.hasSuffix("o"),
            "backspace must remove last typed roman char")
    }

    func test_backspace_afterSpace_clearsTrailingSpaceFlag() {
        let (handler, proxy) = makeHandler()
        type("nhom", into: handler)
        handler.spaceTapped()     // commits, inserts " ", trailingSpace = true
        handler.backspaceTapped() // deletes " ", trailingSpace = false
        handler.returnTapped()    // must NOT double-delete
        handler.textDidChange()

        let text = proxy.text
        XCTAssertTrue(text.hasSuffix("\n"))
        let beforeNewline = text.unicodeScalars.dropLast()
        if let last = beforeNewline.last {
            XCTAssertTrue((0x1780...0x17FF).contains(last.value),
                "char before newline must be Khmer, not space or deleted; got \(text.debugDescription)")
        }
    }

    // MARK: - English mode

    func test_toggleEnglish_setsIsEnglishMode() {
        let (handler, _) = makeHandler()
        handler.toggleEnglish()
        XCTAssertTrue(handler.isEnglishMode)
    }

    func test_toggleEnglish_twice_clearsIsEnglishMode() {
        let (handler, _) = makeHandler()
        handler.toggleEnglish()
        handler.toggleEnglish()
        XCTAssertFalse(handler.isEnglishMode)
    }

    func test_toggleEnglish_firesOnEnglishModeChanged() {
        let (handler, _) = makeHandler()
        var received: [Bool] = []
        handler.onEnglishModeChanged = { received.append($0) }

        handler.toggleEnglish()
        handler.toggleEnglish()

        XCTAssertEqual(received, [true, false])
    }

    func test_sendChar_inEnglishMode_insertsDirectlyWithoutKhmerProcessing() {
        let (handler, proxy) = makeHandler()
        handler.toggleEnglish()

        handler.sendChar("a")

        XCTAssertEqual(proxy.text, "a",
            "English mode must insert roman directly; got \(proxy.text.debugDescription)")
    }

    func test_backspace_inEnglishMode_deletesDirectly() {
        let (handler, proxy) = makeHandler()
        handler.toggleEnglish()
        handler.sendChar("a")

        handler.backspaceTapped()

        XCTAssertEqual(proxy.text, "")
    }

    func test_space_inEnglishMode_insertsSpace() {
        let (handler, proxy) = makeHandler()
        handler.toggleEnglish()

        handler.spaceTapped()

        XCTAssertEqual(proxy.text, " ")
    }

    func test_return_inEnglishMode_insertsNewline() {
        let (handler, proxy) = makeHandler()
        handler.toggleEnglish()

        handler.returnTapped()

        XCTAssertEqual(proxy.text, "\n")
    }

    func test_isEnglishMode_persistsAfterSwitchLayer() {
        let (handler, _) = makeHandler()
        handler.toggleEnglish()

        handler.switchLayer(to: .numeric)

        XCTAssertTrue(handler.isEnglishMode,
            "switching layer must not clear English mode")
    }

    func test_sendChar_onNumericLayer_inEnglishMode_insertsDirectly() {
        let (handler, proxy) = makeHandler()
        handler.toggleEnglish()
        handler.switchLayer(to: .numeric)

        handler.sendChar("1")

        XCTAssertEqual(proxy.text, "1",
            "English mode must bypass Rust session even on numeric layer")
    }

    func test_toggleEnglish_whileComposing_leavesRomanInProxy() {
        let (handler, proxy) = makeHandler()
        type("ka", into: handler)
        let romanInProxy = proxy.text   // "ka" speculatively in the field

        handler.toggleEnglish()

        XCTAssertTrue(proxy.text.hasSuffix(romanInProxy),
            "EN toggle must not delete roman text from proxy; got \(proxy.text.debugDescription)")
    }

    func test_togglePanel_fromEnglishMode_clearsEnglishMode() {
        let (handler, _) = makeHandler()
        handler.toggleEnglish()

        handler.togglePanel()   // no composition in English mode → charPick

        XCTAssertFalse(handler.isEnglishMode,
            "✦ must exit English mode")
    }

    // MARK: - Backspace hold repeat

    // Each repeat tick calls backspaceHoldFired(): proxy.deleteBackward() only —
    // no session call. When the user lifts the finger, backspaceHoldEnded() fires
    // exactly one batched session dispatch for the chars removed from romanBuffer.

    func test_backspaceHoldFired_doesNotDispatchToSession() {
        let dispatcher = CapturingDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()

        handler.backspaceHoldFired()

        XCTAssertNil(dispatcher.capturedSession,
            "backspaceHoldFired must not dispatch to the session queue — only proxy.deleteBackward()")
    }

    func test_backspaceHoldFired_deletesOneCharFromProxy() {
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: SynchronousDispatcher())
        handler.focusIn()
        handler.sendChar("k")   // proxy = "k"

        handler.backspaceHoldFired()

        XCTAssertEqual(proxy.text, "",
            "backspaceHoldFired must delete one char from proxy immediately")
    }

    func test_backspaceHoldEnded_afterHoldFires_dispatchesToSession() {
        let dispatcher = CapturingDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()
        // sendChar puts "k" in both proxy and romanBuffer immediately (proxy insert is
        // synchronous); the session dispatch is captured but not executed.
        handler.sendChar("k")

        handler.backspaceHoldFired()   // romanBuffer = "", pendingHoldBackspaces = 1
        handler.backspaceHoldEnded()   // overwrites capturedSession with hold-end work

        XCTAssertNotNil(dispatcher.capturedSession,
            "backspaceHoldEnded must dispatch one session block after hold fires")
    }

    func test_backspaceHoldEnded_withNoHoldFires_doesNotDispatch() {
        let dispatcher = CapturingDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()

        handler.backspaceHoldEnded()

        XCTAssertNil(dispatcher.capturedSession,
            "backspaceHoldEnded with no hold fires must not dispatch — nothing to sync")
    }

    func test_backspaceHoldFired_pastRomanBuffer_holdEnd_doesNotDispatch() {
        let dispatcher = CapturingDispatcher()
        let proxy = MockTextProxy()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()
        // No typing → romanBuffer is empty; deletions are past the roman buffer

        handler.backspaceHoldFired()
        handler.backspaceHoldEnded()

        XCTAssertNil(dispatcher.capturedSession,
            "hold fires past the roman buffer must not count toward session sync")
    }

    func test_backspaceHoldFired_thenHoldEnded_rendersSessionState() {
        let (handler, _) = makeHandler()
        type("khn", into: handler)   // romanBuffer = "khn"

        var renderCount = 0
        handler.onRender = { _, _ in renderCount += 1 }

        handler.backspaceHoldFired()   // romanBuffer = "kh", no render
        handler.backspaceHoldFired()   // romanBuffer = "k",  no render
        handler.backspaceHoldEnded()   // one batched session → one render

        XCTAssertEqual(renderCount, 1,
            "backspaceHoldEnded must produce exactly one render, not one per hold fire")
    }

    // MARK: - Render coalescing

    func test_sendChar_skipsStaleRenderWhenNewerKeystrokeSupersedesIt() {
        let proxy = MockTextProxy()
        let dispatcher = QueueingDispatcher()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()
        var renderCount = 0
        handler.onRender = { _, _ in renderCount += 1 }

        handler.sendChar("n")   // queues onSession #1
        handler.sendChar("h")   // queues onSession #2 — supersedes #1 before it renders

        dispatcher.sessionBlocks[0]()   // runs session.sendCharacter("n"), queues onMain #1
        dispatcher.sessionBlocks[1]()   // runs session.sendCharacter("h"), queues onMain #2

        dispatcher.mainBlocks[0]()      // stale — must be skipped
        dispatcher.mainBlocks[1]()      // latest — must render

        // 2 optimistic renders (one per sendChar) + 1 deferred render (only the last
        // keystroke's deferred render survives the generation check).
        XCTAssertEqual(renderCount, 3,
            "deferred renders superseded by newer keystrokes are skipped; optimistic renders always fire")
    }

    func test_backspace_emptiesBuffer_callsStripClearBeforeRustReturns() {
        let proxy = MockTextProxy()
        let dispatcher = CapturingDispatcher()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()
        handler.sendChar("k")   // romanBuffer = "k"; CapturingDispatcher captures but does not run

        var stripCleared = false
        handler.onStripClear = { stripCleared = true }

        handler.backspaceTapped()   // romanBuffer becomes "", session block captured (not yet run)

        XCTAssertTrue(stripCleared,
            "onStripClear must fire immediately when the last roman char is deleted — before Rust returns")
    }

    func test_rapidBackspaceTaps_onlyLastDeferredRenderFires() {
        let proxy = MockTextProxy()
        let dispatcher = QueueingDispatcher()
        let handler = KeyboardInputHandler(proxy: proxy, session: KeyboardSession(), dispatcher: dispatcher)
        handler.focusIn()
        // Type "khn", flush session so lastState is set and romanBuffer = "khn"
        handler.sendChar("k"); handler.sendChar("h"); handler.sendChar("n")
        dispatcher.sessionBlocks.forEach { $0() }
        dispatcher.mainBlocks.forEach { $0() }
        dispatcher.sessionBlocks.removeAll(); dispatcher.mainBlocks.removeAll()

        var renderCount = 0
        handler.onRender = { _, _ in renderCount += 1 }

        // Rapid double backspace: "khn" → "kh" → "k" (buffer still non-empty both times)
        handler.backspaceTapped()   // queues session block #0
        handler.backspaceTapped()   // queues session block #1

        dispatcher.sessionBlocks[0]()   // sendBackspace for tap 1 → queues main block #0
        dispatcher.sessionBlocks[1]()   // sendBackspace for tap 2 → queues main block #1

        dispatcher.mainBlocks[0]()      // stale — must be skipped (generation mismatch)
        dispatcher.mainBlocks[1]()      // latest — must render

        // 2 optimistic renders (one per backspace tap) + 1 deferred render (only the last
        // backspace's deferred render survives the generation check).
        XCTAssertEqual(renderCount, 3,
            "deferred backspace renders are coalesced; optimistic renders always fire immediately")
    }
}

// MARK: - Test Doubles

/// Queues session/main blocks without executing them, preserving call order.
/// Lets tests simulate overlapping in-flight dispatches (e.g. two sendChar
/// calls racing) and fire them in any order to test staleness handling.
final class QueueingDispatcher: KeyboardDispatcher {
    var sessionBlocks: [() -> Void] = []
    var mainBlocks: [() -> Void] = []

    func onSession(_ work: @escaping () -> Void) {
        sessionBlocks.append(work)
    }

    func onMain(_ work: @escaping () -> Void) {
        mainBlocks.append(work)
    }
}
