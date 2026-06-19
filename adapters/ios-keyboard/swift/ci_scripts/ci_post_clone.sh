#!/bin/sh

# Xcode Cloud post-clone script.
#
# The iOS project is fully generated and intentionally gitignored:
#   - KhmerIME.xcodeproj            -> xcodegen (from project.yml)
#   - Frameworks/KhmerIME.xcframework -> cargo + xcodebuild -create-xcframework
#   - KhmerIMEKeyboard/Generated/*  -> uniffi-bindgen
#
# Xcode Cloud only clones committed files, so none of these exist at clone
# time. This script reproduces `make platform-build-ios` so that the project,
# the Rust xcframework, and the UniFFI Swift bindings all exist before
# Xcode Cloud resolves packages and builds.

set -e

echo "=== ci_post_clone: bootstrapping iOS build toolchain ==="

# --- Rust toolchain (not preinstalled on Xcode Cloud images) ---
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
# shellcheck source=/dev/null
. "$HOME/.cargo/env"
rustup target add aarch64-apple-ios aarch64-apple-ios-sim

# --- xcodegen (generates KhmerIME.xcodeproj from project.yml) ---
brew install xcodegen

# --- Generate xcframework + UniFFI bindings + the .xcodeproj ---
# CI_PRIMARY_REPOSITORY_PATH is the cloned repo root provided by Xcode Cloud.
cd "$CI_PRIMARY_REPOSITORY_PATH"
make platform-build-ios

echo "=== ci_post_clone: done ==="
