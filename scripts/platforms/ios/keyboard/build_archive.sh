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

# Stamp the Product Version and a build number onto BOTH targets. The app and the
# keyboard extension must carry identical versions or App Store Connect rejects the
# upload, and every upload needs a build number that account has never used before —
# so KHMERIME_IOS_BUILD_NUMBER must be bumped for each one (mirrors
# KHMERIME_ANDROID_VERSION_CODE on the Play side).
BUILD_NUMBER="${KHMERIME_IOS_BUILD_NUMBER:-1}"
VERSION_ARGS=(
	"MARKETING_VERSION=${VERSION}"
	"CURRENT_PROJECT_VERSION=${BUILD_NUMBER}"
)

# 1. xcframework + xcodeproj (canonical build — reused)
make -C "${ROOT_DIR}" platform-build-ios
# 2. archive
mkdir -p "${DIST_DIR}"
xcodebuild -project "${ROOT_DIR}/adapters/ios-keyboard/swift/KhmerIME.xcodeproj" \
	-scheme KhmerIME -configuration Release -destination 'generic/platform=iOS' \
	-archivePath "${ARCHIVE}" "${SIGN_ARGS[@]}" "${VERSION_ARGS[@]}" archive

echo "built ${ARCHIVE} (version ${VERSION}, build ${BUILD_NUMBER})"
echo "  ship: xcodebuild -exportArchive -archivePath ${ARCHIVE} ... (needs a profile; see PACKAGING.md)"
