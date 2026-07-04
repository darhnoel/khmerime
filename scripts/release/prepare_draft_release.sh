#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/release/prepare_draft_release.sh --tag v1.0.0-rc.1 [--build] [--create]

Options:
  --tag TAG    Release tag to stage, for example v1.0.0-rc.1.
  --build      Run `make package` for the current host before collecting artifacts.
  --create     Actually run `gh release create --draft`. Without this, print the command only.

Environment:
  KHMERIME_SIGN_CHECKSUMS=1  GPG-sign SHA256SUMS if gpg is available.
USAGE
}

ROOT_DIR="$(git rev-parse --show-toplevel)"
TAG=""
BUILD=0
CREATE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag)
      TAG="${2:-}"
      shift 2
      ;;
    --build)
      BUILD=1
      shift
      ;;
    --create)
      CREATE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "${TAG}" ]]; then
  echo "--tag is required" >&2
  usage >&2
  exit 2
fi

if [[ "${TAG}" != v* ]]; then
  echo "Release tag should start with v, for example v1.0.0-rc.1" >&2
  exit 2
fi

VERSION="${TAG#v}"
DIST_DIR="${ROOT_DIR}/dist"
RELEASE_DIR="${DIST_DIR}/release/${TAG}"

if [[ "${BUILD}" -eq 1 ]]; then
  make -C "${ROOT_DIR}" package
fi

if git -C "${ROOT_DIR}" rev-parse -q --verify "refs/tags/${TAG}" >/dev/null; then
  :
else
  echo "warning: local tag ${TAG} does not exist yet; create it before final release" >&2
fi

ASSETS=()
while IFS= read -r asset; do
  ASSETS+=("${asset}")
done < <(
  find "${DIST_DIR}" -type f \
    ! -path "${DIST_DIR}/release/*" \
    \( \
      -name '*.pkg' -o \
      -name '*.msi' -o \
      -name '*.deb' -o \
      -name '*.aab' -o \
      -name '*.apk' -o \
      -name '*.zip' \
    \) | sort
)

if [[ "${#ASSETS[@]}" -eq 0 ]]; then
  echo "No release assets found under ${DIST_DIR}." >&2
  echo "Run a package target first, for example: make linux-package" >&2
  exit 1
fi

mkdir -p "${RELEASE_DIR}"

for asset in "${ASSETS[@]}"; do
  cp -f "${asset}" "${RELEASE_DIR}/"
done

(
  cd "${RELEASE_DIR}"
  rm -f SHA256SUMS SHA256SUMS.sig
  for file in *; do
    [[ -f "${file}" ]] || continue
    [[ "${file}" == "SHA256SUMS" || "${file}" == "SHA256SUMS.sig" ]] && continue
    shasum -a 256 "${file}"
  done > SHA256SUMS
)

if [[ "${KHMERIME_SIGN_CHECKSUMS:-0}" == "1" ]]; then
  if command -v gpg >/dev/null 2>&1; then
    gpg --batch --yes --detach-sign --output "${RELEASE_DIR}/SHA256SUMS.sig" "${RELEASE_DIR}/SHA256SUMS"
  else
    echo "warning: KHMERIME_SIGN_CHECKSUMS=1 but gpg is not installed" >&2
  fi
fi

RELEASE_FILES=()
while IFS= read -r file; do
  RELEASE_FILES+=("${file}")
done < <(find "${RELEASE_DIR}" -maxdepth 1 -type f | sort)

TITLE="KhmerIME ${VERSION}"
DEB_VERSION="${VERSION/-rc./~rc.}"
if [[ "${DEB_VERSION}" != *-* ]]; then
  DEB_VERSION="${DEB_VERSION}-1"
fi
NOTES="Draft release candidate for KhmerIME ${VERSION}.

Artifacts are unsigned unless the filename or store pipeline says otherwise.
Linux Debian package version: ${DEB_VERSION}."
CMD=(gh release create "${TAG}" --draft --title "${TITLE}" --notes "${NOTES}")
if [[ "${TAG}" == *-rc.* ]]; then
  CMD+=(--prerelease --latest=false)
fi
CMD+=("${RELEASE_FILES[@]}")

echo "Staged release assets:"
printf '  %s\n' "${RELEASE_FILES[@]}"
echo
echo "Draft release command:"
printf '  %q' "${CMD[@]}"
echo

if [[ "${CREATE}" -eq 1 ]]; then
  if ! command -v gh >/dev/null 2>&1; then
    echo "gh is required for --create. Install GitHub CLI and run gh auth login." >&2
    exit 2
  fi
  "${CMD[@]}"
fi
