struct KeyboardLayerVisibility {
    let showsQwerty: Bool
    let showsNumeric: Bool
    let showsSymbols: Bool
    let showsPanel: Bool
    let showsCandidateRow: Bool

    init(state: KeyboardState) {
        showsQwerty = state == .qwerty
        showsNumeric = state == .numeric
        showsSymbols = state == .symbols
        showsPanel = state == .panel || state == .charPick
        showsCandidateRow = state != .panel && state != .charPick
    }
}
