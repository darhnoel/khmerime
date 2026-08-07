import Foundation

// MacosModelArming
// ================
// The socket for a span-proposal provider (see docs/model-provider-challenge.md). This OSS build
// ships a no-op: `armIfNeeded()` returns false, so the engine stays on Standard (deterministic
// lexicon) decoding. A build that has a provider REPLACES this one file — the paid macOS build's
// project spec excludes this stub and substitutes an arming implementation that loads a model and
// calls into the provider. Everything else in the app is the code you are reading.
//
// The controller calls `MacosModelArming.armIfNeeded()` once at startup; in this repo that call
// does nothing.
enum MacosModelArming {
    @discardableResult
    static func armIfNeeded() -> Bool { false }
}
