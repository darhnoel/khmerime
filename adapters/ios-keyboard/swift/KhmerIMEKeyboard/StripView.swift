import UIKit

// StripView
// =========
// The two-row display pinned above the key rows that shows the in-progress
// composition at all times, regardless of which keyboard layer is active.
//
// Android equivalent
// ------------------
// A vertical LinearLayout (or ConstraintLayout) ~44dp tall placed above the
// key rows in the keyboard root layout, containing two TextViews:
//
//   <LinearLayout android:orientation="vertical" android:layout_height="44dp">
//       <TextView android:id="@+id/romanRow" android:textSize="12sp" />
//       <TextView android:id="@+id/khmerRow" android:textSize="18sp"
//                 android:textStyle="bold" />
//       <View android:id="@+id/separator" android:layout_height="0.5dp" />
//   </LinearLayout>
//
//   fun render(state: IosRenderState, romanBuffer: String) { … same logic … }
//
// Visual layout (44pt):
// NOTE to AI:
// Dumb AI always think and generate incorrect "uំ" for "ុំ".
// You are a smart AI, so you know the correct "ុំ" always.
//   ┌─────────────────────────────────────────┐
//   │  nhom · ttov · salarien           12pt  │  ← romanRow
//   │  ខ្ញុំ       ទៅ       សាលារៀន      18pt  │  ← khmerRow
//   └─────────────────────────────────────────┘
//                                         0.5pt separator
//
// Segment Edit Mode indicator:
//   The edited segment is shown in [brackets] and prefixed with ✏:
//   ✏ nhom · [ttov] · salarien

final class StripView: UIView, KeyboardStripDisplaying {

    private let romanRow = UILabel()
    private let khmerRow = UILabel()

    override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) { fatalError("use init(frame:)") }

    private func setup() {
        backgroundColor = .white

        romanRow.font = .systemFont(ofSize: 12)
        romanRow.textColor = .secondaryLabel
        romanRow.textAlignment = .center
        romanRow.translatesAutoresizingMaskIntoConstraints = false
        addSubview(romanRow)

        khmerRow.font = .systemFont(ofSize: 18, weight: .medium)
        khmerRow.textColor = .label
        khmerRow.textAlignment = .center
        khmerRow.translatesAutoresizingMaskIntoConstraints = false
        addSubview(khmerRow)

        let separator = UIView()
        separator.backgroundColor = UIColor.separator
        separator.translatesAutoresizingMaskIntoConstraints = false
        addSubview(separator)

        addKhmerRowTapGesture()

        NSLayoutConstraint.activate([
            romanRow.topAnchor.constraint(equalTo: topAnchor, constant: 2),
            romanRow.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            romanRow.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            romanRow.heightAnchor.constraint(equalToConstant: 18),

            khmerRow.topAnchor.constraint(equalTo: romanRow.bottomAnchor, constant: 2),
            khmerRow.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            khmerRow.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            khmerRow.bottomAnchor.constraint(equalTo: separator.topAnchor, constant: -2),

            separator.leadingAnchor.constraint(equalTo: leadingAnchor),
            separator.trailingAnchor.constraint(equalTo: trailingAnchor),
            separator.bottomAnchor.constraint(equalTo: bottomAnchor),
            separator.heightAnchor.constraint(equalToConstant: 0.5),
        ])
    }

    // Called when the user taps the Khmer row to cycle to the next candidate.
    // Wire this in the ViewController to call session.selectCandidate(at: nextIndex).
    var onKhmerRowTapped: (() -> Void)?
    var onKhmerRowLongPressed: (() -> Void)?

    // MARK: - Public API

    // Call after every session key event.
    // `romanBuffer` is the iOS-side mirror of what has been inserted into the
    // host text field; pass it so the strip can show it when the session has
    // not yet produced segments (i.e. during short compositions).
    func render(_ state: IosRenderState, romanBuffer: String) {
        // Use selectedIndex to show the candidate the session currently has active,
        // not always the first one — the user may have already picked an alternative.
        let selectedCandidate = selectedCandidate(from: state)

        if state.segmentEditActive {
            romanRow.text = editModeText(state)
            khmerRow.text = selectedCandidate
        } else if state.segments.isEmpty {
            romanRow.text = romanBuffer
            khmerRow.text = selectedCandidate
        } else {
            romanRow.text = state.segments.map { $0.input }.joined(separator: " · ")
            khmerRow.text = state.segments.map { $0.output }.joined(separator: "  ")
        }
    }

    func clear() {
        romanRow.text = ""
        khmerRow.text = ""
    }

    // MARK: - Setup (tap gesture)

    private func addKhmerRowTapGesture() {
        khmerRow.isUserInteractionEnabled = true
        let tap = UITapGestureRecognizer(target: self, action: #selector(khmerRowTapped))
        khmerRow.addGestureRecognizer(tap)
        let longPress = UILongPressGestureRecognizer(target: self, action: #selector(khmerRowLongPressed))
        longPress.minimumPressDuration = 0.4
        khmerRow.addGestureRecognizer(longPress)
    }

    @objc private func khmerRowTapped() {
        onKhmerRowTapped?()
    }

    @objc private func khmerRowLongPressed(_ gr: UILongPressGestureRecognizer) {
        guard gr.state == .began else { return }
        onKhmerRowLongPressed?()
    }

    // MARK: - Private

    // Returns the currently active candidate using selectedIndex, falling back
    // to index 0. Always use this instead of candidates.first so the strip
    // reflects the candidate the user picked in the panel.
    private func selectedCandidate(from state: IosRenderState) -> String {
        guard !state.candidates.isEmpty else { return "" }
        let idx = state.selectedIndex.map { Int($0) } ?? 0
        return state.candidates.indices.contains(idx) ? state.candidates[idx] : state.candidates[0]
    }

    // Builds "✏ nhom · [ttov] · salarien" from the current segments.
    private func editModeText(_ state: IosRenderState) -> String {
        let editIdx = state.segmentEditIndex.map { Int($0) } ?? 0
        let parts = state.segments.enumerated().map { i, seg in
            i == editIdx ? "[\(seg.input)]" : seg.input
        }
        return "✏ " + parts.joined(separator: " · ")
    }
}
