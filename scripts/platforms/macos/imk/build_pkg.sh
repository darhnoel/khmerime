#!/usr/bin/env bash
# Build the macOS input-method installer (.pkg) into dist/macos/.
#
# Default (no signing config): UNSIGNED .pkg — reproducible, no credentials.
# macOS 26 (Tahoe) Gatekeeper rejects unsigned input methods, so to actually
# ship you need the signed path below.
#
# Signed path (opt-in): if the caller exports both a Developer ID Application
# identity (MACOS_CODE_SIGN_IDENTITY) and a Developer ID Installer identity
# (MACOS_INSTALLER_SIGN_IDENTITY) — the Makefile passes these from
# macos-signing.local.mk — the app is codesigned + hardened, the installer is
# productsigned, then the .pkg is notarized and stapled. Same recipe the
# `platform-reinstall-macos` install flow already proves out. See PACKAGING.md.
set -euo pipefail

ROOT_DIR="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
VERSION="${KHMERIME_PACKAGE_VERSION:-$(awk -F '"' '/^version = / { print $2; exit }' "${ROOT_DIR}/Cargo.toml")}"
DIST_DIR="${ROOT_DIR}/dist/macos"
BUILD_DIR="/tmp/khmerime-macos-build"
PROJECT="${ROOT_DIR}/adapters/macos-imk/swift/KhmerIMEMacOS.xcodeproj"
APP="${BUILD_DIR}/Build/Products/Release/KhmerIMEMacOS.app"

APP_IDENTITY="${MACOS_CODE_SIGN_IDENTITY:-}"
INSTALLER_IDENTITY="${MACOS_INSTALLER_SIGN_IDENTITY:-}"
NOTARY_PROFILE="${MACOS_NOTARY_PROFILE:-khmerime-notary}"
ENTITLEMENTS="${MACOS_ENTITLEMENTS:-${ROOT_DIR}/adapters/macos-imk/swift/KhmerIMEMacOS/KhmerIMEMacOS.entitlements}"

# Sign only when both identities are set and non-placeholder ('<...>' is the
# committed example value). Anything else falls back to the unsigned build.
sign=0
if [ -n "${APP_IDENTITY}" ] && [ -n "${INSTALLER_IDENTITY}" ] \
	&& [[ "${APP_IDENTITY}" != *"<"* ]] && [[ "${INSTALLER_IDENTITY}" != *"<"* ]]; then
	sign=1
fi

command -v xcodebuild >/dev/null 2>&1 || { echo "xcodebuild (Xcode) required" >&2; exit 2; }

# 1. xcframework (canonical build — reused, not reimplemented)
make -C "${ROOT_DIR}" platform-build-macos
mkdir -p "${DIST_DIR}"

if [ "${sign}" -eq 1 ]; then
	PKG="${DIST_DIR}/KhmerIME-${VERSION}-macos.pkg"
	# Build with the exact args notarization needs: manual/ad-hoc, no injected
	# get-task-allow (CODE_SIGN_INJECT_BASE_ENTITLEMENTS=NO), then re-sign.
	xcodebuild -project "${PROJECT}" -scheme KhmerIMEMacOS -configuration Release \
		-derivedDataPath "${BUILD_DIR}" \
		CODE_SIGN_STYLE=Manual CODE_SIGN_IDENTITY=- CODE_SIGN_INJECT_BASE_ENTITLEMENTS=NO build
	codesign --force --deep --options runtime --timestamp \
		--entitlements "${ENTITLEMENTS}" --sign "${APP_IDENTITY}" "${APP}"
	codesign --verify --deep --strict --verbose=2 "${APP}"
	# Sign the installer with the Developer ID Installer identity, then notarize
	# the .pkg (covers the app inside) and staple the ticket for offline Gatekeeper.
	productbuild --component "${APP}" "/Library/Input Methods" --sign "${INSTALLER_IDENTITY}" "${PKG}"
	xcrun notarytool submit "${PKG}" --keychain-profile "${NOTARY_PROFILE}" --wait
	xcrun stapler staple "${PKG}"
	xcrun stapler validate "${PKG}"
	pkgutil --check-signature "${PKG}"
	spctl --assess --type install --verbose=2 "${PKG}"
	echo "built ${PKG} (signed + notarized + stapled)"
else
	PKG="${DIST_DIR}/KhmerIME-${VERSION}-macos-unsigned.pkg"
	# unsigned: signing is the ship step, not the build step
	xcodebuild -project "${PROJECT}" -scheme KhmerIMEMacOS -configuration Release \
		-derivedDataPath "${BUILD_DIR}" CODE_SIGNING_ALLOWED=NO build
	productbuild --component "${APP}" "/Library/Input Methods" "${PKG}"
	echo "built ${PKG}"
	echo "  ship: set MACOS_INSTALLER_SIGN_IDENTITY in macos-signing.local.mk for a signed .pkg (see PACKAGING.md)"
fi
