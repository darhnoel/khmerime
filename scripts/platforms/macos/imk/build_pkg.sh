#!/usr/bin/env bash
# Build the macOS input-method installer (.pkg) into dist/macos/.
#
# Produces an UNSIGNED .pkg by default — reproducible, no credentials. macOS 26
# (Tahoe) Gatekeeper rejects unsigned input methods, so to actually ship, sign +
# notarize the .app first (the `platform-reinstall-macos` flow already does this
# with your Developer ID) — see PACKAGING.md.
set -euo pipefail

ROOT_DIR="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
VERSION="${KHMERIME_PACKAGE_VERSION:-$(awk -F '"' '/^version = / { print $2; exit }' "${ROOT_DIR}/Cargo.toml")}"
DIST_DIR="${ROOT_DIR}/dist/macos"
BUILD_DIR="/tmp/khmerime-macos-build"
APP="${BUILD_DIR}/Build/Products/Release/KhmerIMEMacOS.app"
PKG="${DIST_DIR}/KhmerIME-${VERSION}-macos-unsigned.pkg"

command -v xcodebuild >/dev/null 2>&1 || { echo "xcodebuild (Xcode) required" >&2; exit 2; }

# 1. xcframework + xcodeproj (canonical build — reused, not reimplemented)
make -C "${ROOT_DIR}" platform-build-macos
# 2. build the .app unsigned (signing is the ship step, not the build step)
xcodebuild -project "${ROOT_DIR}/adapters/macos-imk/swift/KhmerIMEMacOS.xcodeproj" \
	-scheme KhmerIMEMacOS -configuration Release -derivedDataPath "${BUILD_DIR}" \
	CODE_SIGNING_ALLOWED=NO build
# 3. wrap the app into an installer that drops it in /Library/Input Methods
mkdir -p "${DIST_DIR}"
productbuild --component "${APP}" "/Library/Input Methods" "${PKG}"

echo "built ${PKG}"
echo "  ship: sign+notarize the .app, then re-run productbuild (see PACKAGING.md)"
