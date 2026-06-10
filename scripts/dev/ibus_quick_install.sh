#!/bin/bash

set -euo pipefail

make data-build
make data-check
make ibus-install

# Clear cached dictionary image so the bridge rebuilds on next keystroke.
cache_dir="${HOME}/.cache/khmerime"
if [ -d "$cache_dir" ]; then
  rm -f "$cache_dir"/*.bin
fi

if command -v ibus &>/dev/null && ibus engine &>/dev/null; then
  ibus restart
fi
