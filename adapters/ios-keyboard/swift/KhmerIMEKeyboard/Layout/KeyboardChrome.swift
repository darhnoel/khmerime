import Foundation

struct QuickAccessItem: Equatable {
    let displayText: String
    let commitText: String
    let accessibilityLabel: String?

    init(_ displayText: String, commitText: String? = nil, accessibilityLabel: String? = nil) {
        self.displayText = displayText
        self.commitText = commitText ?? displayText
        self.accessibilityLabel = accessibilityLabel
    }
}

enum QuickAccessSpec {
    static let digits = "១២៣៤៥៦៧៨៩០".map { QuickAccessItem(String($0)) }

    static let marks = [
        QuickAccessItem("។", accessibilityLabel: "Khmer full stop"),
        QuickAccessItem("៕", accessibilityLabel: "Khmer final period"),
        QuickAccessItem("៖", accessibilityLabel: "Khmer sign camnuc pii kuuh"),
        QuickAccessItem("ៈ", accessibilityLabel: "Yukaleapintu"),
        QuickAccessItem("ៗ", accessibilityLabel: "Khmer repetition sign"),
        QuickAccessItem("៘"),
        QuickAccessItem("៙"),
        QuickAccessItem("៚"),
        QuickAccessItem("៛", accessibilityLabel: "Khmer currency symbol riel"),
        // Apple's Khmer shaper supplies the placeholder circle for isolated
        // nonspacing marks. An explicit ◌ would render a second circle.
        QuickAccessItem("៊"), QuickAccessItem("័"), QuickAccessItem("៌"), QuickAccessItem("៍"),
        QuickAccessItem("៏"), QuickAccessItem("៎"), QuickAccessItem("៑"),
    ]
}

// KeyboardChrome
// ==============
// Decides which input chrome rows are worth their height. Roman composition always
// owns the strip while composing; the candidate row is reserved only when its
// current surface has visible content.

enum KeyboardChrome {
    enum Presentation: Equatable {
        case hidden
        case charPickQuickAccess
        case charPickCandidates
        case quickAccess
        case composition

        var rowCount: Int {
            switch self {
            case .hidden:
                return 0
            case .charPickQuickAccess, .charPickCandidates:
                return 1
            case .quickAccess, .composition:
                return 2
            }
        }

        var rows: Rows {
            switch self {
            case .hidden:
                return .none
            case .charPickQuickAccess, .charPickCandidates:
                return .candidateOnly
            case .quickAccess, .composition:
                return .stripAndCandidate
            }
        }
    }

    static func presentation(
        isEnglish: Bool,
        keyboardState: KeyboardState,
        romanHint: String,
        state: IosRenderState
    ) -> Presentation {
        if isEnglish { return .hidden }
        if keyboardState == .charPick {
            return state.candidates.isEmpty ? .charPickQuickAccess : .charPickCandidates
        }
        let isComposing = !romanHint.isEmpty || !state.segments.isEmpty || !state.preedit.isEmpty
        return isComposing ? .composition : .quickAccess
    }

    enum Rows: Equatable {
        case none
        case stripOnly
        case candidateOnly
        case stripAndCandidate
    }

}
