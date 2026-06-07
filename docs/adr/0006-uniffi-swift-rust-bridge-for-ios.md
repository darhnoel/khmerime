# UniFFI as the Swift/Rust bridge for the iOS keyboard extension

The iOS keyboard extension must be written in Swift (`UIInputViewController` is an
Objective-C class that cannot be subclassed from Rust), but the IME engine lives in
`khmerime_session` (Rust). A bridge is required. We chose **UniFFI** over cbindgen
and swift-bridge.

## The key decisions

- **UniFFI, not cbindgen.** cbindgen requires hand-maintained `extern "C"` wrappers
  for every function and manual Swift bridging-header updates whenever the Rust API
  changes. UniFFI generates type-safe Swift wrappers from annotated Rust — adding a new
  method or enum variant means rerunning `uniffi-bindgen generate`, not editing a
  C header. The session contract already defines enums (`IosKeyEvent`) and structs
  (`IosRenderState`, `IosSegmentEntry`) that cbindgen cannot represent without
  flattening; UniFFI maps them to Swift enums and classes directly.

- **UniFFI, not swift-bridge.** swift-bridge is newer and less battle-tested.
  UniFFI has been in production use in Firefox iOS, Mozilla VPN, and Signal since 2021.
  The API is stable; swift-bridge's was still changing at decision time.

- **Mirror types at the adapter boundary; do not annotate `crates/session`.** The
  exported Swift surface is `KhmerIMESession`, `IosKeyEvent`, `IosRenderState`, and
  `IosSegmentEntry` — all defined in `adapters/ios-keyboard`. `crates/session` types
  (`NativeKeyEvent`, `SessionSnapshot`, `SegmentPreviewEntry`) stay annotation-free.
  This keeps the shared crate free of iOS-specific tooling and is consistent with how
  `adapters/windows-tsf` defines its own `WindowsRenderState` rather than exposing
  session internals.

- **`KhmerIMESession` is the single Swift-visible session handle.** It wraps
  `ImeSession` internally and exposes four methods: `focusIn()`, `focusOut()`,
  `processKey(event:)`, and `setCursorLocation(x:y:width:height:)`. Each returns a
  fresh `IosRenderState`. Swift never calls `ImeSession` directly and never sees
  `NativeKeyEvent` or `SessionCommand`.

- **XCFramework is a build artifact, not committed to git.** `make platform-build-ios`
  compiles the Rust staticlib for `aarch64-apple-ios` and `aarch64-apple-ios-sim`,
  runs `uniffi-bindgen generate`, and assembles
  `adapters/ios-keyboard/swift/Frameworks/KhmerIME.xcframework`. The Xcode project
  references this path. Developers run `make platform-build-ios` before opening Xcode,
  the same way Linux developers run `make ibus-install` before using IBus.

## What this constrains

1. `adapters/ios-keyboard/Cargo.toml` must add `crate-type = ["staticlib"]` and a
   `uniffi` dependency. A `build.rs` calls `uniffi::generate_scaffolding`.
2. The Makefile needs a `platform-build-ios` target that chains: `cargo build` for both
   iOS targets, `uniffi-bindgen generate`, and `xcodebuild -create-xcframework`.
3. `adapters/ios-keyboard/swift/Frameworks/` must be gitignored.
4. Changing the exported API (adding a method, a new `IosKeyEvent` variant, a new field
   on `IosRenderState`) requires rerunning `uniffi-bindgen generate` before building the
   Xcode project. A missing regeneration step breaks the Swift build. The `make
   platform-build-ios` target must always run bindgen so it cannot be skipped.

## When to revisit

- If UniFFI's generated Swift API becomes a maintenance burden and a hand-written
  C bridge proves simpler for the actual surface we expose.
- If we later share the iOS engine library with a macOS IMK adapter — at that point
  the XCFramework targets and `make` targets would need to be unified.
