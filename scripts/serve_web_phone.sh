#!/usr/bin/env bash
set -euo pipefail

ADDR="${ADDR:-0.0.0.0}"
PORT="${PORT:-4173}"

exec dx serve --platform web --addr "$ADDR" --port "$PORT" --open false
