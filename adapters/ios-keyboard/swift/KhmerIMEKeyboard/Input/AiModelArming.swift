import Foundation

// AiModelArming
// =============
// Seam for registering an optional span-proposal provider at keyboard launch. The OSS build ships
// this NO-OP: no provider, so Smart mode stays inert. A closed build replaces this file (via its
// private project spec) with an implementation that points the provider at bundled resources and
// registers it. Provider-agnostic: the public seam names no model.
//
// Called once from KeyboardViewController before the session's Smart preference is applied.
enum AiModelArming {
    // Returns whether a provider was registered. Always false in the OSS build.
    @discardableResult
    static func armIfNeeded() -> Bool { false }
}
