#!/usr/bin/env bash
set -euo pipefail

ADDR="${ADDR:-0.0.0.0}"
PORT="${PORT:-4173}"
FEATURES="${DX_FEATURES:-}"
VERBOSE="${DX_VERBOSE:-}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP_DIR="$ROOT_DIR/apps/dioxus-app"
INDEX_HTML="$ROOT_DIR/target/dx/roman_lookup/debug/web/public/index.html"
PUBLIC_ASSETS_DIR="$ROOT_DIR/target/dx/roman_lookup/debug/web/public/assets"
HEAD_SNIPPET="$ROOT_DIR/assets/web_preboot_head.html"
BODY_SNIPPET="$ROOT_DIR/assets/web_preboot_body.html"

sync_static_assets() {
  mkdir -p "$PUBLIC_ASSETS_DIR"

  if [[ -f "$ROOT_DIR/assets/main.css" ]]; then
    cp "$ROOT_DIR/assets/main.css" "$PUBLIC_ASSETS_DIR/main.css"
  fi
  if [[ -d "$ROOT_DIR/assets/css" ]]; then
    rm -rf "$PUBLIC_ASSETS_DIR/css"
    cp -R "$ROOT_DIR/assets/css" "$PUBLIC_ASSETS_DIR/css"
  fi
  if [[ -d "$ROOT_DIR/assets/vendor" ]]; then
    rm -rf "$PUBLIC_ASSETS_DIR/vendor"
    cp -R "$ROOT_DIR/assets/vendor" "$PUBLIC_ASSETS_DIR/vendor"
  fi
}

inject_shell_splash() {
  local index_html="$1"
  if [[ ! -f "$index_html" ]] || grep -q 'id="app-preboot-splash"' "$index_html"; then
    return 0
  fi

  local tmp_file
  tmp_file="$(mktemp)"
  awk -v head_snippet="$HEAD_SNIPPET" -v body_snippet="$BODY_SNIPPET" '
    /<\/head>/ && !inserted_head {
      while ((getline line < head_snippet) > 0) {
        print line
      }
      close(head_snippet)
      inserted_head = 1
    }
    /<div id="main"><\/div>/ && !inserted_body {
      while ((getline line < body_snippet) > 0) {
        print line
      }
      close(body_snippet)
      inserted_body = 1
    }
    { print }
  ' "$index_html" > "$tmp_file"
  mv "$tmp_file" "$index_html"
}

watch_and_patch_shell() {
  local parent_pid="$1"
  while kill -0 "$parent_pid" 2>/dev/null; do
    sync_static_assets
    inject_shell_splash "$INDEX_HTML"
    sleep 0.25
  done
}

DX_CMD=(dx serve --platform web --addr "$ADDR" --port "$PORT" --open false)
if [[ -n "$FEATURES" ]]; then
  DX_CMD+=(--features "$FEATURES")
fi
if [[ -n "$VERBOSE" ]]; then
  DX_CMD+=(--verbose)
fi

echo "Starting Dioxus web server on ${ADDR}:${PORT}"
echo "Local URL: http://127.0.0.1:${PORT}"
if [[ "$ADDR" == "0.0.0.0" ]]; then
  echo "Phone/LAN URL: http://<this-computer-ip>:${PORT}"
fi
echo "Using app directory: $APP_DIR"
echo "Syncing CSS assets into: $PUBLIC_ASSETS_DIR"
echo "Press Ctrl+C to stop."

sync_static_assets
watch_and_patch_shell "$$" &
PATCH_PID=$!

cleanup() {
  kill "$PATCH_PID" 2>/dev/null || true
}

trap cleanup EXIT INT TERM

(
  cd "$APP_DIR"
  "${DX_CMD[@]}"
)
