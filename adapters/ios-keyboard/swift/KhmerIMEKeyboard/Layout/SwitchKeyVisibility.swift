/// Decides visibility of the bottom-row next-keyboard globe and the English
/// toggle. Both are ALWAYS visible (ADR-0022): the globe must be present on
/// every device to satisfy App Store Guideline 4.4.1, and EN is an independent
/// feature. `needsInputModeSwitchKey` is accepted only to document that it is
/// deliberately NOT used to gate the globe — that gating caused the rejection.
struct SwitchKeyVisibility {
    let globeHidden: Bool
    let englishHidden: Bool

    init(needsInputModeSwitchKey: Bool) {
        _ = needsInputModeSwitchKey // intentionally ignored; see ADR-0022
        globeHidden = false
        englishHidden = false
    }
}
