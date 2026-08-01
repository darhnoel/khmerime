#!/usr/bin/env bash
# Export a signed .ipa from the iOS archive, and optionally upload it to
# App Store Connect (TestFlight / App Store review).
#
# Runs after build_archive.sh. Two config files gate the two halves, following the
# repo convention that an absent config degrades instead of failing:
#
#   adapters/ios-keyboard/ExportOptions.plist        (from ExportOptions.example.plist)
#     absent → stop after reporting the archive; nothing to export against.
#   adapters/ios-keyboard/appstore-upload.local.sh   (from appstore-upload.example.sh)
#     absent → export the .ipa and stop; upload by hand via Xcode's Organizer.
#
# Requires an **Apple Distribution** certificate plus App Store provisioning profiles
# for BOTH bundle IDs — the container app and the keyboard extension. A missing appex
# profile is the classic failure here, and xcodebuild's error names the bundle ID.
set -euo pipefail

ROOT_DIR="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
VERSION="${KHMERIME_PACKAGE_VERSION:-$(awk -F '"' '/^version = / { print $2; exit }' "${ROOT_DIR}/Cargo.toml")}"
DIST_DIR="${ROOT_DIR}/dist/ios"
ARCHIVE="${DIST_DIR}/KhmerIME-${VERSION}-ios.xcarchive"
EXPORT_OPTIONS="${ROOT_DIR}/adapters/ios-keyboard/ExportOptions.plist"
UPLOAD_CONFIG="${ROOT_DIR}/adapters/ios-keyboard/appstore-upload.local.sh"
IPA="${DIST_DIR}/KhmerIME.ipa"

command -v xcodebuild >/dev/null 2>&1 || { echo "xcodebuild (Xcode) required" >&2; exit 2; }

if [ ! -d "${ARCHIVE}" ]; then
	echo "error: no archive at ${ARCHIVE} — run 'make ios-package' first." >&2
	exit 2
fi

if [ ! -f "${EXPORT_OPTIONS}" ]; then
	cat >&2 <<EOF
Archive built, but no export options — skipping .ipa export.

To export a signed .ipa:
  1. Create an **Apple Distribution** certificate
     (Xcode → Settings → Accounts → Manage Certificates → + → Apple Distribution).
  2. cp adapters/ios-keyboard/ExportOptions.example.plist \\
       adapters/ios-keyboard/ExportOptions.plist
  3. Re-run: make ios-release
EOF
	exit 0
fi

echo "==> exporting .ipa"
rm -rf "${DIST_DIR}/export"
xcodebuild -exportArchive \
	-archivePath "${ARCHIVE}" \
	-exportOptionsPlist "${EXPORT_OPTIONS}" \
	-exportPath "${DIST_DIR}/export" \
	-allowProvisioningUpdates

# xcodebuild names the .ipa after the scheme; normalise so downstream steps and the
# release workflow can reference a stable path.
FOUND_IPA="$(find "${DIST_DIR}/export" -maxdepth 1 -name '*.ipa' | head -1)"
[ -n "${FOUND_IPA}" ] || { echo "error: export produced no .ipa" >&2; exit 1; }
mv "${FOUND_IPA}" "${IPA}"
echo "exported ${IPA}"

if [ ! -f "${UPLOAD_CONFIG}" ]; then
	cat >&2 <<EOF

Signed .ipa is ready; no upload config, so stopping here.

Upload it either way:
  • Xcode → Window → Organizer → Distribute App, or
  • cp adapters/ios-keyboard/appstore-upload.example.sh \\
      adapters/ios-keyboard/appstore-upload.local.sh   (then fill in the API key)
    and re-run: make ios-release
EOF
	exit 0
fi

# shellcheck source=/dev/null
. "${UPLOAD_CONFIG}"
: "${KHMERIME_ASC_KEY_ID:?set KHMERIME_ASC_KEY_ID in ${UPLOAD_CONFIG}}"
: "${KHMERIME_ASC_ISSUER_ID:?set KHMERIME_ASC_ISSUER_ID in ${UPLOAD_CONFIG}}"
case "${KHMERIME_ASC_KEY_ID}${KHMERIME_ASC_ISSUER_ID}" in
	*"<"*) echo "error: replace the placeholder values in ${UPLOAD_CONFIG}" >&2; exit 2 ;;
esac

echo "==> validating before upload"
xcrun altool --validate-app -f "${IPA}" --type ios \
	--apiKey "${KHMERIME_ASC_KEY_ID}" --apiIssuer "${KHMERIME_ASC_ISSUER_ID}"

echo "==> uploading to App Store Connect"
xcrun altool --upload-app -f "${IPA}" --type ios \
	--apiKey "${KHMERIME_ASC_KEY_ID}" --apiIssuer "${KHMERIME_ASC_ISSUER_ID}"

echo "uploaded. Processing takes a few minutes; the build then appears in TestFlight."
