#!/usr/bin/env bash
set -euo pipefail

ADDR="${ADDR:-0.0.0.0}"
PORT="${PORT:-4173}"
FEATURES="${DX_FEATURES:-}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT_DIR/apps/dioxus-app"
INDEX_HTML="$ROOT_DIR/target/dx/roman_lookup/debug/web/public/index.html"
HEAD_SNIPPET="$ROOT_DIR/assets/web_preboot_head.html"
BODY_SNIPPET="$ROOT_DIR/assets/web_preboot_body.html"

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
  while kill -0 "$DX_PID" 2>/dev/null; do
    inject_shell_splash "$INDEX_HTML"
    sleep 0.25
  done
}

DX_CMD=(dx serve --platform web --addr "$ADDR" --port "$PORT" --open false)
if [[ -n "$FEATURES" ]]; then
  DX_CMD+=(--features "$FEATURES")
fi

(
  cd "$APP_DIR"
  "${DX_CMD[@]}"
) &
DX_PID=$!
watch_and_patch_shell &
PATCH_PID=$!

cleanup() {
  kill "$PATCH_PID" 2>/dev/null || true
  kill "$DX_PID" 2>/dev/null || true
}

trap cleanup EXIT INT TERM

wait "$DX_PID"
