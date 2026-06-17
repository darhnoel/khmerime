struct KeyboardLayerVisibility {
    let showsQwerty: Bool
    let showsNumeric: Bool
    let showsSymbols: Bool
    let showsCandidateRow: Bool

    init(state: KeyboardState) {
        showsQwerty = state == .qwerty || state == .charPick
        showsNumeric = state == .numeric
        showsSymbols = state == .symbols
        showsCandidateRow = true
    }
}
