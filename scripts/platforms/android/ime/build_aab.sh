#!/usr/bin/env bash
# Build the Android release App Bundle (.aab) into dist/android/.
#
# The .aab is unsigned — Google Play re-signs on upload, so no keystore is needed
# to produce a valid upload artifact. For local sideloading you'd build a signed
# .apk instead (see PACKAGING.md).
set -euo pipefail

ROOT_DIR="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
VERSION="${KHMERIME_PACKAGE_VERSION:-$(awk -F '"' '/^version = / { print $2; exit }' "${ROOT_DIR}/Cargo.toml")}"
ADAPTER="${ROOT_DIR}/adapters/android-ime"
ABI="${ANDROID_ABI:-arm64-v8a}"
JAVA_HOME_DIR="${ANDROID_JAVA_HOME:-${JAVA_HOME:-/Applications/Android Studio.app/Contents/jbr/Contents/Home}}"
DIST_DIR="${ROOT_DIR}/dist/android"

command -v cargo-ndk >/dev/null 2>&1 || { echo "cargo-ndk required: cargo install cargo-ndk" >&2; exit 2; }

# Gradle reads the Product Version from this env (versionName inside the .aab);
# export the resolved value so the bundle internals always agree with the filename.
export KHMERIME_PACKAGE_VERSION="${VERSION}"

# 1. logo assets + native lib (release)
make -C "${ROOT_DIR}" platform-android-assets
cargo ndk -t "${ABI}" -o "${ADAPTER}/app/src/main/jniLibs" build -p khmerime_android_ime --release
# 2. release bundle
cd "${ADAPTER}"
JAVA_HOME="${JAVA_HOME_DIR}" PATH="${JAVA_HOME_DIR}/bin:${PATH}" ./gradlew :app:bundleRelease
mkdir -p "${DIST_DIR}"
AAB="$(find app/build/outputs/bundle/release -name '*.aab' | head -1)"
[ -n "${AAB}" ] || { echo "no .aab produced — check gradle release config" >&2; exit 1; }
cp "${AAB}" "${DIST_DIR}/KhmerIME-${VERSION}-android.aab"

echo "built ${DIST_DIR}/KhmerIME-${VERSION}-android.aab"
echo "  ship: upload to Play Console (Play signs it); or sign an .apk for sideload (see PACKAGING.md)"
