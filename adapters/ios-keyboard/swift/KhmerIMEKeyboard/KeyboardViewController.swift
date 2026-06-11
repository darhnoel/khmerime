import UIKit

// KeyboardViewController
// ======================
// Orchestrates the Khmer IME keyboard extension. Owns the state machine,
// roman buffer, session lifecycle, and the render loop.
//
// Android equivalent
// ------------------
// This class maps directly to an Android InputMethodService subclass:
//
//   class KhmerIMEService : InputMethodService(), CandidatePanelListener {
//       private val session = KeyboardSession()
//       private var romanBuffer = StringBuilder()
//       private var keyboardState = KeyboardState.QWERTY
//
//       override fun onCreateInputView(): View { … build root layout … }
//       override fun onStartInput(…) { session.focusIn() }
//       override fun onFinishInput() { session.focusOut() }
//
//       // Render loop — identical logic:
//       private fun render(state: IosRenderState) {
//           stripView.render(state, romanBuffer.toString())
//           if (keyboardState == KeyboardState.PANEL) panelView.render(state)
//       }
//   }
//
// Keyboard states
// ---------------
//   .qwerty    Default roman-input view. 💡 in shift slot, 123/space/./⏎ bottom.
//   .numeric   123 layer: 1–0, punctuation, #+=, ABC/space/⏎.
//   .symbols   #+= layer: []{}#%^*+=, currencies, 123/space/⏎.
//   .panel     💡 candidate panel: chips + candidates + bottom row.
//   .charPick  CharPick mode: panel visible with A–Z chip row + candidate collection.
//              Tap a letter chip → candidates shown; tap a candidate → commits and
//              resets to alphabet. ⌫ cancels lookup. 💡 exits CharPick entirely.
//
// State transitions:
//   qwerty ⇄ numeric ⇄ symbols         (123 / #+= / ABC keys)
//   qwerty ⇄ panel                     (💡 when composition is active, or ✏ chip edit)
//   qwerty ⇄ charPick                  (💡 when no composition is active)
//
// Roman-buffer contract (see KeyboardSession.swift for full explanation)
// -----------------------------------------------------------------------
// Roman chars are inserted into the host text field speculatively. On ⏎ they
// are bulk-deleted and the committed Khmer text is inserted instead. The
// buffer also mirrors what to delete when the session auto-commits (e.g.
// a digit key outside composition → Khmer digit).
// In CharPick mode the roman buffer is always empty — roman input is a lookup
// filter only and is never inserted into the host text field.

private enum KeyboardState { case qwerty, numeric, symbols, panel, charPick }

class KeyboardViewController: UIInputViewController {

    // MARK: - State

    private var keyboardState: KeyboardState = .qwerty
    private let session = KeyboardSession()

    // Mirrors the roman characters currently in the host text field for this
    // composition. Reset to "" on every commit or full backspace.
    private var romanBuffer = ""

    // Cached render state; used by the panel and chip-tap navigation.
    private var lastState: IosRenderState?

    // MARK: - Views

    private var stripView:   StripView!
    private var panelView:   CandidatePanelView!
    private var qwertyView:  UIView!
    private var numericView: UIView!
    private var symbolsView: UIView!
    private var panelBottomRow: UIStackView!

    // MARK: - Lifecycle

    override func viewDidLoad() {
        super.viewDidLoad()
        setupLayout()
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        let state = session.focusIn()
        render(state)
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        _ = session.focusOut()
    }

    // Detects external text changes (e.g., ✖ clear button in search bars).
    // If the text before the cursor no longer ends with our roman buffer,
    // something cleared it externally — reset composition and strip.
    override func textDidChange(_ textInput: UITextInput?) {
        guard !romanBuffer.isEmpty else { return }
        let before = textDocumentProxy.documentContextBeforeInput ?? ""
        guard !before.hasSuffix(romanBuffer) else { return }
        romanBuffer = ""
        _ = session.sendReturn()
        stripView.clear()
        if keyboardState == .panel || keyboardState == .charPick {
            transition(to: .qwerty)
        }
    }

    // MARK: - Layout

    private func setupLayout() {
        // UIInputViewController requires a height constraint with priority < 1000
        // to avoid conflicts with the system's own sizing.
        let h = view.heightAnchor.constraint(equalToConstant: 260)
        h.priority = UILayoutPriority(999)
        h.isActive = true

        let root = UIView()
        root.backgroundColor = UIColor.systemGray5
        root.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(root)
        NSLayoutConstraint.activate([
            root.topAnchor.constraint(equalTo: view.topAnchor),
            root.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            root.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            root.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        ])

        stripView = StripView()
        stripView.translatesAutoresizingMaskIntoConstraints = false
        stripView.onKhmerRowTapped = { [weak self] in self?.cycleCandidate() }

        panelView = CandidatePanelView()
        panelView.delegate = self
        panelView.translatesAutoresizingMaskIntoConstraints = false

        qwertyView  = buildQwertyView()
        numericView = buildNumericView()
        symbolsView = buildSymbolsView()

        panelBottomRow = makeBottomRow(leftLabel: "123", leftAction: #selector(numericTapped), includePeriod: true)
        panelBottomRow.translatesAutoresizingMaskIntoConstraints = false

        for v in [stripView!, qwertyView!, numericView!, symbolsView!, panelView!, panelBottomRow!] {
            v.translatesAutoresizingMaskIntoConstraints = false
            root.addSubview(v)
        }

        NSLayoutConstraint.activate([
            // Strip is always visible at the top.
            stripView.topAnchor.constraint(equalTo: root.topAnchor),
            stripView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            stripView.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            stripView.heightAnchor.constraint(equalToConstant: 44),

            // QWERTY / 123 / #+= each fill the space below the strip.
            qwertyView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            qwertyView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            qwertyView.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            qwertyView.bottomAnchor.constraint(equalTo: root.bottomAnchor),

            numericView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            numericView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            numericView.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            numericView.bottomAnchor.constraint(equalTo: root.bottomAnchor),

            symbolsView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            symbolsView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            symbolsView.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            symbolsView.bottomAnchor.constraint(equalTo: root.bottomAnchor),

            // Panel occupies the chip + candidate rows; the bottom row sits below.
            panelView.topAnchor.constraint(equalTo: stripView.bottomAnchor),
            panelView.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            panelView.trailingAnchor.constraint(equalTo: root.trailingAnchor),

            panelBottomRow.topAnchor.constraint(equalTo: panelView.bottomAnchorGuide.topAnchor, constant: 8),
            panelBottomRow.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 3),
            panelBottomRow.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -3),
            panelBottomRow.heightAnchor.constraint(equalToConstant: 44),
        ])

        transition(to: .qwerty)
    }

    // MARK: - State Machine

    private func transition(to state: KeyboardState) {
        keyboardState = state
        qwertyView.isHidden  = state != .qwerty
        numericView.isHidden = state != .numeric
        symbolsView.isHidden = state != .symbols
        // CharPick shows the panel (A–Z chips + candidate collection) instead of QWERTY.
        let inPanel = state == .panel || state == .charPick
        panelView.isHidden      = !inPanel
        panelBottomRow.isHidden = !inPanel
    }

    // MARK: - Render Loop

    // Called after every session key event. Updates the strip and, when in
    // panel state, rebuilds the chip and candidate rows.
    private func render(_ state: IosRenderState) {
        lastState = state
        stripView.render(state, romanBuffer: romanBuffer)
        if keyboardState == .panel || keyboardState == .charPick { panelView.render(state) }
    }

    // MARK: - Character Input

    // Unified handler for all printable key taps (letters, digits, symbols).
    //
    // The roman char is speculatively inserted into the host text field so the
    // user sees immediate feedback. If the session returns a commitText (e.g.
    // a digit outside composition → Khmer digit), the speculative roman char is
    // deleted and the committed Khmer text is inserted in its place.
    private func sendChar(_ ch: String) {
        // CharPick uses the panel's alphabet chips, not QWERTY keys.
        guard keyboardState != .charPick else { return }
        textDocumentProxy.insertText(ch)
        romanBuffer += ch
        let state = session.sendCharacter(ch)
        if let committed = state.commitText, !committed.isEmpty {
            for _ in romanBuffer { textDocumentProxy.deleteBackward() }
            textDocumentProxy.insertText(committed)
            romanBuffer = ""
        }
        render(state)
    }

    // MARK: - Key Actions

    // Cycles to the next candidate when the user taps the Khmer strip row.
    // Works for both flat compositions (single word, multiple candidates)
    // and segmented compositions (moves to next candidate for the focused segment).
    private func cycleCandidate() {
        guard let current = lastState, !current.candidates.isEmpty else { return }
        let currentIdx = current.selectedIndex.map { Int($0) } ?? 0
        let nextIdx = (currentIdx + 1) % current.candidates.count
        let state = session.selectCandidate(at: nextIdx)
        render(state)
    }

    @objc func letterTapped(_ sender: UIButton) {
        guard let ch = sender.title(for: .normal)?.lowercased(), !ch.isEmpty else { return }
        sendChar(ch)
    }

    @objc func symbolKeyTapped(_ sender: UIButton) {
        guard let ch = sender.title(for: .normal), !ch.isEmpty else { return }
        sendChar(ch)
    }

    @objc func periodTapped() { sendChar(".") }

    @objc func backspaceTapped() {
        if keyboardState == .charPick {

            if let current = lastState, !current.candidates.isEmpty {
                // Cancel the current letter lookup and go back to the alphabet picker.
                _ = session.enterCharPick()
                lastState = nil
                stripView.clear()
                panelView.renderCharPickAlphabet()
            } else {
                // Already at alphabet — delete the last committed Khmer character.
                textDocumentProxy.deleteBackward()
            }
            return
        }
        if !romanBuffer.isEmpty { romanBuffer.removeLast() }
        textDocumentProxy.deleteBackward()
        let state = session.sendBackspace()
        render(state)
        if romanBuffer.isEmpty { stripView.clear() }
    }

    @objc func spaceTapped() {
        romanBuffer += " "
        textDocumentProxy.insertText(" ")
        render(session.sendSpace())
    }

    @objc func returnTapped() {
        if keyboardState == .charPick {
            if let current = lastState, !current.candidates.isEmpty {
                textDocumentProxy.insertText(current.candidates[0])
                _ = session.enterCharPick()
                lastState = nil
                stripView.clear()
                panelView.renderCharPickAlphabet()
            }

            return
        }
        let state = session.sendReturn()

        // Build the Khmer commit string from segments (concatenated, no spaces —
        // Khmer script convention) or from commitText for single-word compositions.
        let khmerText = state.segments.isEmpty
            ? (state.commitText ?? "")
            : state.segments.map { $0.output }.joined()

        if !khmerText.isEmpty {
            for _ in romanBuffer { textDocumentProxy.deleteBackward() }
            textDocumentProxy.insertText(khmerText)
        }

        romanBuffer = ""
        stripView.clear()
        if keyboardState == .panel { transition(to: .qwerty) }
    }

    @objc func togglePanelTapped() {
        switch keyboardState {
        case .panel:
            transition(to: .qwerty)
        case .charPick:
            // 💡 on the qwerty row while charPick is active (shouldn't normally occur
            // since qwerty is hidden, but handle it defensively).
            _ = session.exitCharPick()

            stripView.clear()
            transition(to: .qwerty)
        default:
            // With active composition: show word candidate panel.
            // Without composition: enter CharPick directly.
            let hasComposition = lastState.map { !$0.candidates.isEmpty } ?? false
            if hasComposition {
                if let state = lastState { panelView.render(state) }
                transition(to: .panel)
            } else {
                _ = session.enterCharPick()
                lastState = nil
                stripView.clear()
                panelView.renderCharPickAlphabet()
                transition(to: .charPick)
            }
        }
    }

    // MARK: - Layer Switches

    @objc func numericTapped() { transition(to: .numeric) }
    @objc func symbolsTapped() { transition(to: .symbols) }
    @objc func abcTapped()     { transition(to: .qwerty)  }
}

// MARK: - CandidatePanelDelegate

extension KeyboardViewController: CandidatePanelDelegate {

    func candidatePanel(_ panel: CandidatePanelView, didTapChipAt index: Int) {
        guard let current = lastState else { return }
        let focused = current.focusedSegmentIndex.map { Int($0) } ?? 0
        let diff = index - focused
        var state = current
        if diff > 0      { for _ in 0..<diff    { state = session.sendRight() } }
        else if diff < 0 { for _ in 0..<(-diff) { state = session.sendLeft()  } }
        render(state)
    }

    func candidatePanel(_ panel: CandidatePanelView, didRequestEditAt index: Int) {
        guard let current = lastState else { return }
        // Navigate focus to the requested segment, then enter Segment Edit Mode.
        let focused = current.focusedSegmentIndex.map { Int($0) } ?? 0
        let diff = index - focused
        if diff > 0      { for _ in 0..<diff    { _ = session.sendRight() } }
        else if diff < 0 { for _ in 0..<(-diff) { _ = session.sendLeft()  } }
        let state = session.sendTab()
        render(state)
        // Return to QWERTY so the user can retype the segment's roman input.
        // The strip shows "✏ nhom · [ttov] · salarien" as a visual indicator.
        transition(to: .qwerty)
    }

    func candidatePanelDidEnterCharPick(_ panel: CandidatePanelView) {
        for _ in romanBuffer { textDocumentProxy.deleteBackward() }
        romanBuffer = ""
        _ = session.enterCharPick()
        lastState = nil
        stripView.clear()
        panelView.renderCharPickAlphabet()
        transition(to: .charPick)
    }

    func candidatePanel(_ panel: CandidatePanelView, didTapCharPickLetter letter: Character) {
        let state = session.sendCharacter(String(letter))
        lastState = state
        stripView.render(state, romanBuffer: String(letter))
        panelView.render(state)
    }

    func candidatePanel(_ panel: CandidatePanelView, didSelectCandidateAt index: Int) {
        if keyboardState == .charPick {
            if let candidate = lastState?.candidates[index] {
                textDocumentProxy.insertText(candidate)
            }

            _ = session.enterCharPick()
            lastState = nil
            stripView.clear()
            panelView.renderCharPickAlphabet()
            return
        }
        let state = session.selectCandidate(at: index)
        render(state)
    }

    func candidatePanelDidDismiss(_ panel: CandidatePanelView) {
        if keyboardState == .charPick {
            _ = session.exitCharPick()

            stripView.clear()
        }
        transition(to: .qwerty)
    }
}
