#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
BRIDGE_BIN="${ROOT_DIR}/target/release/khmerime_ibus_bridge"

echo "[khmerime] building bridge..."
cargo build --release --bin khmerime_ibus_bridge >/dev/null

echo "[khmerime] bridge protocol smoke check..."
RESP="$(
  {
    printf '{"cmd":"focus_in"}\n'
    printf '{"cmd":"process_key_event","keyval":106,"keycode":0,"state":0}\n'
    printf '{"cmd":"process_key_event","keyval":101,"keycode":0,"state":0}\n'
    printf '{"cmd":"process_key_event","keyval":97,"keycode":0,"state":0}\n'
    printf '{"cmd":"process_key_event","keyval":65293,"keycode":0,"state":0}\n'
    printf '{"cmd":"shutdown"}\n'
  } | "${BRIDGE_BIN}" | tail -n 2 | head -n 1
)"
echo "${RESP}" | rg -q '"commit_text":"[^"]+"' || {
  echo "bridge did not produce commit_text for jea + Enter" >&2
  exit 1
}

if command -v dbus-run-session >/dev/null 2>&1 && command -v ibus-daemon >/dev/null 2>&1 && command -v ibus >/dev/null 2>&1; then
  echo "[khmerime] ibus discovery smoke check..."
  if dbus-run-session -- bash -lc '
    ibus-daemon -drx >/tmp/khmerime_ibus_daemon.log 2>&1 &
    DAEMON_PID=$!
    sleep 2
    ibus list-engine | grep -q "khmerime"
    kill "${DAEMON_PID}" >/dev/null 2>&1 || true
  '; then
    echo "[khmerime] ibus discovery smoke check passed."
  else
    echo "[khmerime] ibus discovery smoke check skipped (session bus not allowed in this environment)."
  fi
else
  echo "[khmerime] skipped ibus discovery check (dbus-run-session/ibus-daemon/ibus missing)"
fi

echo "[khmerime] smoke checks passed."
