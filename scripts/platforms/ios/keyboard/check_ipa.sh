#!/usr/bin/env bash
# Refuse an .ipa that lacks a file the store build must carry.
#
#   check_ipa.sh <ipa> [<path inside Payload/> ...]
#
# export_ipa.sh runs this before uploading, with KHMERIME_IPA_REQUIRE from the git-ignored
# upload config. When the App Store build differs from the open-source one, list here what
# only the store build has: a build made the wrong way then stops on this Mac, where
# xcodebuild and altool would otherwise both report success and upload it.
set -euo pipefail

IPA="${1:?usage: check_ipa.sh <ipa> [required path ...]}"
shift

[ -f "${IPA}" ] || { echo "error: no .ipa at ${IPA}" >&2; exit 2; }
[ "$#" -gt 0 ] || exit 0

listing="$(unzip -Z1 "${IPA}")"
missing=0
for path in "$@"; do
	if ! printf '%s\n' "${listing}" | grep -qxF "Payload/${path}"; then
		echo "error: ${path} is not in the .ipa" >&2
		missing=1
	fi
done

if [ "${missing}" != 0 ]; then
	echo "error: this is not the store build; refusing to upload ${IPA}" >&2
	exit 3
fi
echo "ipa holds every required file ($#)"
