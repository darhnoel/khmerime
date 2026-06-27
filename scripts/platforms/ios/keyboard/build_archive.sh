#!/usr/bin/env bash
# Build an iOS archive (.xcarchive) into dist/ios/.
#
# Produces an UNSIGNED archive — reproducible, no provisioning profile. Exporting
# a shippable .ipa needs a profile for BOTH the container app and the keyboard
# appex; do that with `xcodebuild -exportArchive` + your ExportOptions.plist (see
# PACKAGING.md). If Xcode automatic signing is already set up from device
# installs, set KHMERIME_IOS_SIGN=1 to archive signed instead.
set -euo pipefail

ROOT_DIR="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
VERSION="${KHMERIME_PACKAGE_VERSION:-$(awk -F '"' '/^version = / { print $2; exit }' "${ROOT_DIR}/Cargo.toml")}"
DIST_DIR="${ROOT_DIR}/dist/ios"
ARCHIVE="${DIST_DIR}/KhmerIME-${VERSION}-ios.xcarchive"

command -v xcodebuild >/dev/null 2>&1 || { echo "xcodebuild (Xcode) required" >&2; exit 2; }

SIGN_ARGS=(CODE_SIGNING_ALLOWED=NO)
[ "${KHMERIME_IOS_SIGN:-0}" = "1" ] && SIGN_ARGS=(-allowProvisioningUpdates)

# 1. xcframework + xcodeproj (canonical build — reused)
make -C "${ROOT_DIR}" platform-build-ios
# 2. archive
mkdir -p "${DIST_DIR}"
xcodebuild -project "${ROOT_DIR}/adapters/ios-keyboard/swift/KhmerIME.xcodeproj" \
	-scheme KhmerIME -configuration Release -destination 'generic/platform=iOS' \
	-archivePath "${ARCHIVE}" "${SIGN_ARGS[@]}" archive

echo "built ${ARCHIVE}"
echo "  ship: xcodebuild -exportArchive -archivePath ${ARCHIVE} ... (needs a profile; see PACKAGING.md)"
