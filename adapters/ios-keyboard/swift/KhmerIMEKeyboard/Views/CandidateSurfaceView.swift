import UIKit

// CandidateSurfaceView
// ====================
// Hosts the candidate-row slot (ADR-0014): the Phrase Wheel during normal
// composition, the word-level CandidateRowView during CharPick (and, later,
// Level-2 editing). Conforms to KeyboardCandidateRowDisplaying so it drops into
// the existing slot, and forwards each selection callback to the right child —
// wheel scrolling → `onPhraseSelected`, CharPick tap → `onCandidateSelected`.

final class CandidateSurfaceView: UIView, KeyboardCandidateRowDisplaying {

    let wheel = PhraseWheelView()
    let candidateRow = CandidateRowView()

    var onPhraseSelected: ((Int) -> Void)? {
        get { wheel.onPhraseSelected }
        set { wheel.onPhraseSelected = newValue }
    }

    var onCandidateSelected: ((Int) -> Void)? {
        get { candidateRow.onCandidateSelected }
        set { candidateRow.onCandidateSelected = newValue }
    }

    override init(frame: CGRect) {
        super.init(frame: frame)
        for child in [wheel, candidateRow] as [UIView] {
            child.translatesAutoresizingMaskIntoConstraints = false
            addSubview(child)
            NSLayoutConstraint.activate([
                child.leadingAnchor.constraint(equalTo: leadingAnchor),
                child.trailingAnchor.constraint(equalTo: trailingAnchor),
                child.topAnchor.constraint(equalTo: topAnchor),
                child.bottomAnchor.constraint(equalTo: bottomAnchor),
            ])
        }
        candidateRow.isHidden = true
    }

    required init?(coder: NSCoder) { fatalError("use init(frame:)") }

    func render(_ state: IosRenderState, presentation: CandidateRowPresentation = .composition) {
        switch presentation {
        case .composition where state.segmentEditActive:
            // Level 2: editing one word — the focused segment's candidates take over.
            wheel.isHidden = true
            candidateRow.isHidden = false
            candidateRow.render(state, presentation: .composition)
        case .composition:
            candidateRow.isHidden = true
            wheel.render(state, presentation: presentation)
            // Hide the row entirely when there are no alternatives — the strip's
            // preview already shows the top reading, so it stands alone (ADR-0015).
            wheel.isHidden = !wheel.hasAlternatives
        case .charPick:
            wheel.isHidden = true
            candidateRow.isHidden = false
            candidateRow.render(state, presentation: presentation)
        }
    }

    func showQuickAccess(_ items: [QuickAccessItem], onSelected: @escaping (QuickAccessItem) -> Void) {
        wheel.isHidden = true
        candidateRow.isHidden = false
        candidateRow.showQuickAccess(items, onSelected: onSelected)
    }

    func clear() {
        wheel.clear()
        candidateRow.clear()
    }
}
