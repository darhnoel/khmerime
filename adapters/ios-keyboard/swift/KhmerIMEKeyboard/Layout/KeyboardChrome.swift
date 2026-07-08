import Foundation

// KeyboardChrome
// ==============
// Decides which input chrome rows are worth their height. Roman composition always
// owns the strip while composing; the candidate row is reserved only when its
// current surface has visible content.

enum KeyboardChrome {
    enum Rows: Equatable {
        case none
        case stripOnly
        case candidateOnly
        case stripAndCandidate
    }

    static func rows(for keyboardState: KeyboardState, romanHint: String, state: IosRenderState) -> Rows {
        if keyboardState == .charPick {
            return state.candidates.isEmpty ? .none : .candidateOnly
        }

        let hasStripContent = !romanHint.isEmpty || !state.segments.isEmpty || !state.preedit.isEmpty
        guard hasStripContent else { return .none }

        if state.segmentEditActive {
            return state.candidates.isEmpty ? .stripOnly : .stripAndCandidate
        }

        let selectedPhraseIndex = Int(state.selectedPhraseIndex)
        let hasPhraseAlternatives = state.phraseCandidates.indices.contains { $0 != selectedPhraseIndex }
        return hasPhraseAlternatives ? .stripAndCandidate : .stripOnly
    }
}
