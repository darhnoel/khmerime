import Foundation

// KeyboardChrome
// ==============
// Decides which input chrome rows are worth their height. Roman composition owns
// the strip + candidate row; CharPick owns only the candidate row and only while
// candidates exist.

enum KeyboardChrome {
    enum Rows: Equatable {
        case none
        case candidateOnly
        case stripAndCandidate
    }

    static func rows(for keyboardState: KeyboardState, romanHint: String, state: IosRenderState) -> Rows {
        if keyboardState == .charPick {
            return state.candidates.isEmpty ? .none : .candidateOnly
        }
        return (!romanHint.isEmpty || !state.candidates.isEmpty) ? .stripAndCandidate : .none
    }
}
