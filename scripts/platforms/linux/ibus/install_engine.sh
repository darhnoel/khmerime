#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
INSTALL_LIBEXEC_DIR="${KHMERIME_IBUS_LIBEXEC_DIR:-/usr/libexec/khmerime}"
INSTALL_COMPONENT_DIR="${KHMERIME_IBUS_COMPONENT_DIR:-/usr/share/ibus/component}"
ENGINE_SCRIPT_SRC="${ROOT_DIR}/adapters/linux-ibus/python/khmerime_ibus_engine.py"
ENGINE_HELPER_SRC="${ROOT_DIR}/adapters/linux-ibus/python/ibus_segment_preview.py"
ENGINE_SCRIPT_DST="${INSTALL_LIBEXEC_DIR}/khmerime-ibus-engine"
ENGINE_HELPER_DST="${INSTALL_LIBEXEC_DIR}/ibus_segment_preview.py"
BRIDGE_BINARY_DST="${INSTALL_LIBEXEC_DIR}/khmerime-ibus-bridge"
COMPONENT_XML_PATH="${INSTALL_COMPONENT_DIR}/khmerime.xml"
LEGACY_USER_COMPONENT_PATH="${HOME}/.local/share/ibus/component/khmerime.xml"

run_install() {
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

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required" >&2
  exit 2
fi

COMPONENT_WRITE_TARGET="$(nearest_existing_dir "${INSTALL_COMPONENT_DIR}")"
LIBEXEC_WRITE_TARGET="$(nearest_existing_dir "${INSTALL_LIBEXEC_DIR}")"

if [[ -w "${COMPONENT_WRITE_TARGET}" && -w "${LIBEXEC_WRITE_TARGET}" ]]; then
  NEED_SUDO="0"
else
  NEED_SUDO="1"
  if ! command -v sudo >/dev/null 2>&1; then
    echo "Need write access to ${INSTALL_COMPONENT_DIR} and ${INSTALL_LIBEXEC_DIR}." >&2
    echo "Run as root or install sudo." >&2
    exit 2
  fi
fi

echo "[khmerime] building khmerime_ibus_bridge (release)..."
cargo build --release --bin khmerime_ibus_bridge

echo "[khmerime] preparing install directories..."
run_install install -d "${INSTALL_LIBEXEC_DIR}" "${INSTALL_COMPONENT_DIR}"

echo "[khmerime] installing bridge + ibus adapter..."
run_install install -m 0755 "${ROOT_DIR}/target/release/khmerime_ibus_bridge" "${BRIDGE_BINARY_DST}"
run_install install -m 0755 "${ENGINE_SCRIPT_SRC}" "${ENGINE_SCRIPT_DST}"
run_install install -m 0644 "${ENGINE_HELPER_SRC}" "${ENGINE_HELPER_DST}"

echo "[khmerime] writing IBus component XML..."
TMP_COMPONENT_XML="$(mktemp)"
cat > "${TMP_COMPONENT_XML}" <<EOF
<component>
    <name>org.freedesktop.IBus.KhmerIME</name>
    <description>AngkorIME input method engine</description>
    <version>0.1.0</version>
    <license>MIT</license>
    <author>AngkorIME contributors</author>
    <homepage>https://github.com/darhnoel/angkorime</homepage>
    <textdomain>khmerime</textdomain>
    <exec>${ENGINE_SCRIPT_DST} --ibus --bridge-path ${BRIDGE_BINARY_DST}</exec>
    <engines>
        <engine>
            <name>khmerime</name>
            <longname>AngkorIME</longname>
            <description>Khmer romanization IME powered by AngkorIME</description>
            <language>km</language>
            <license>MIT</license>
            <author>AngkorIME contributors</author>
            <icon></icon>
            <layout>us</layout>
            <symbol>ខ</symbol>
            <rank>80</rank>
        </engine>
    </engines>
</component>
EOF
run_install install -m 0644 "${TMP_COMPONENT_XML}" "${COMPONENT_XML_PATH}"
rm -f "${TMP_COMPONENT_XML}"

if [[ -f "${LEGACY_USER_COMPONENT_PATH}" ]]; then
  echo "[khmerime] removing legacy user component at ${LEGACY_USER_COMPONENT_PATH}"
  rm -f "${LEGACY_USER_COMPONENT_PATH}"
fi

echo "[khmerime] refreshing IBus cache..."
if command -v ibus >/dev/null 2>&1; then
  if ibus address >/dev/null 2>&1; then
    ibus write-cache || true
    ibus restart || true
  else
    echo "[khmerime] no active IBus session bus detected; skipped ibus write-cache/restart."
    echo "[khmerime] open a GNOME terminal in your desktop session or log out/in, then run: ibus write-cache && ibus restart"
  fi
else
  echo "[khmerime] ibus command not found; install ibus and rerun cache/restart manually" >&2
fi

echo "[khmerime] install complete."
if command -v ibus >/dev/null 2>&1; then
  if ibus address >/dev/null 2>&1; then
    if ibus list-engine | grep -q "khmerime"; then
      echo "[khmerime] engine is visible in ibus list-engine."
    else
      echo "[khmerime] WARNING: engine is not in ibus list-engine yet."
      echo "Try: ibus restart"
      echo "If still missing, log out and log back in."
    fi
  else
    echo "[khmerime] skipped ibus list-engine check (no active IBus session bus)."
  fi
fi
echo "Next: Settings -> Keyboard -> Input Sources -> search for Khmer and add KhmerIME."
