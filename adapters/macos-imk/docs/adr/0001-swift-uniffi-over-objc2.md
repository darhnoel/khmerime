# Swift + UniFFI over pure Rust (objc2)

The macOS IMK host shell is written in Swift, calling a UniFFI-exported Rust library —
the same pattern used by the iOS keyboard adapter (root ADR-0006). The alternative was
`objc2-input-method-kit` (0.3.2), which would let Rust subclass `IMKInputController`
directly, mirroring how Windows TSF uses `windows-rs` to call COM APIs from Rust.

`objc2-input-method-kit` was rejected because the crate is auto-generated from Apple
headers and its coverage of the IMK edge cases (cursor anchor, marked text interaction,
composition cancellation) is unverified. Swift is the supported first-class language for
InputMethodKit and has extensive real-world IME examples. The UniFFI build toolchain is
already proven in this repo; reusing it adds no new risk. The tradeoff is a codegen build
step (`uniffi-bindgen` + xcodegen) that the pure-Rust path would avoid, but that cost is
already paid for iOS and the Makefile target can be modelled directly on
`platform-build-ios`.
