#!/usr/bin/env bash
set -euo pipefail

INSTALL_LIBEXEC_DIR="${KHMERIME_IBUS_LIBEXEC_DIR:-/usr/libexec/khmerime}"
INSTALL_COMPONENT_PATH="${KHMERIME_IBUS_COMPONENT_DIR:-/usr/share/ibus/component}/khmerime.xml"
LEGACY_USER_LIBEXEC_DIR="${HOME}/.local/libexec/khmerime"
LEGACY_USER_COMPONENT_PATH="${HOME}/.local/share/ibus/component/khmerime.xml"

run_rm() {
  if [[ "${NEED_SUDO}" == "1" ]]; then
    sudo "$@"
  else
    "$@"
  fi
}

nearest_existing_dir() {
  local path="$1"
  while [[ ! -d "${path}" ]]; do
    local parent
    parent="$(dirname "${path}")"
    if [[ "${parent}" == "${path}" ]]; then
      break
    fi
    path="${parent}"
  done
  printf "%s\n" "${path}"
}

COMPONENT_WRITE_TARGET="$(nearest_existing_dir "$(dirname "${INSTALL_COMPONENT_PATH}")")"
LIBEXEC_WRITE_TARGET="$(nearest_existing_dir "${INSTALL_LIBEXEC_DIR}")"

if [[ -w "${COMPONENT_WRITE_TARGET}" && -w "${LIBEXEC_WRITE_TARGET}" ]]; then
  NEED_SUDO="0"
else
  NEED_SUDO="1"
  if ! command -v sudo >/dev/null 2>&1; then
    echo "Need write access to remove ${INSTALL_COMPONENT_PATH} and ${INSTALL_LIBEXEC_DIR}." >&2
    echo "Run as root or install sudo." >&2
    exit 2
  fi
fi

echo "[khmerime] removing IBus engine files..."
run_rm rm -f "${INSTALL_LIBEXEC_DIR}/khmerime-ibus-engine"
run_rm rm -f "${INSTALL_LIBEXEC_DIR}/ibus_segment_preview.py"
run_rm rm -f "${INSTALL_LIBEXEC_DIR}/khmerime-ibus-bridge"
run_rm rmdir "${INSTALL_LIBEXEC_DIR}" 2>/dev/null || true
run_rm rm -f "${INSTALL_COMPONENT_PATH}"

rm -f "${LEGACY_USER_COMPONENT_PATH}"
rm -f "${LEGACY_USER_LIBEXEC_DIR}/khmerime-ibus-engine"
rm -f "${LEGACY_USER_LIBEXEC_DIR}/ibus_segment_preview.py"
rm -f "${LEGACY_USER_LIBEXEC_DIR}/khmerime-ibus-bridge"
rmdir "${LEGACY_USER_LIBEXEC_DIR}" 2>/dev/null || true

echo "[khmerime] refreshing IBus cache..."
if command -v ibus >/dev/null 2>&1; then
  if ibus address >/dev/null 2>&1; then
    ibus write-cache || true
    ibus restart || true
  else
    echo "[khmerime] no active IBus session bus detected; skipped ibus write-cache/restart."
  fi
else
  echo "[khmerime] ibus command not found; skip cache/restart" >&2
fi

echo "[khmerime] uninstall complete."
