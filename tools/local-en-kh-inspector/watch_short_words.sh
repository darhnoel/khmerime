#!/usr/bin/env bash
# Watch data/most-common-en-kh.csv and regenerate short-common-en-words.txt
# on every modification. Lines emitted as "<abs path>:<lineno>  <en> -> <kh>"
# so they are clickable in Zed regardless of cwd.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
SRC="${ROOT_DIR}/data/most-common-en-kh.csv"
OUT="${ROOT_DIR}/short-common-en-words.txt"

if [[ ! -f "$SRC" ]]; then
  echo "error: $SRC not found" >&2
  exit 1
fi

regenerate() {
  awk -F, -v src="$SRC" 'length($1)<=3 {printf "%s:%d  %-4s -> %s\n", src, NR, $1, $2}' \
    "$SRC" > "$OUT"
  echo "[$(date +%T)] regenerated $OUT ($(wc -l < "$OUT") lines)"
}

regenerate

if command -v entr >/dev/null 2>&1; then
  export SRC OUT
  export -f regenerate
  echo "$SRC" | entr -n bash -c regenerate
elif command -v inotifywait >/dev/null 2>&1; then
  while inotifywait -qq -e modify,close_write "$SRC"; do
    regenerate
  done
else
  echo "error: install 'entr' or 'inotify-tools' to enable watching" >&2
  exit 1
fi
