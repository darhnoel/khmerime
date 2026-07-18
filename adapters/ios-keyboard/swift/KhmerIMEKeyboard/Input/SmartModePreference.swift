import Foundation

// SmartModePreference
// ===================
// Persists the user's Standard/Smart choice in a shared App Group UserDefaults suite, so the
// host-app Settings toggle (writer) and the keyboard extension (reader) see the same value.
// Standard (the default) is lexicon + fuzzy only; Smart enables the injected span-proposal
// provider — inert without a registered provider, so in the OSS build the toggle has no visible
// effect. Provider-agnostic: names no model. Mirrors the Android SmartModePreference.

struct SmartModePreference {

    // The App Group shared between the app and keyboard extension (see project.yml entitlements).
    static let suiteName = "group.com.khmerime"
    private static let key = "smart_mode"

    private let defaults: UserDefaults

    // `defaults` is injectable for tests; production uses the shared App Group suite.
    init(defaults: UserDefaults = UserDefaults(suiteName: SmartModePreference.suiteName) ?? .standard) {
        self.defaults = defaults
    }

    var isEnabled: Bool {
        defaults.bool(forKey: SmartModePreference.key)
    }

    func setEnabled(_ enabled: Bool) {
        defaults.set(enabled, forKey: SmartModePreference.key)
    }
}
