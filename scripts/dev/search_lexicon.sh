#!/bin/bash

set -euo pipefail

if [[ $# -lt 1 || -z "${1:-}" ]]; then
  echo "usage: $0 <pattern> [chunks|chunk-number|chunk-file|path]" >&2
  exit 2
fi

scope="${2:-chunks}"

if [[ "$scope" == "chunks" ]]; then
  target="data/lexicon/chunks"
elif [[ "$scope" =~ ^[0-9]+$ ]]; then
  printf -v chunk_file "chunk_%04d.csv" "$scope"
  target="data/lexicon/chunks/${chunk_file}"
elif [[ "$scope" == chunk_*.csv ]]; then
  target="data/lexicon/chunks/${scope}"
elif [[ -e "$scope" ]]; then
  target="$scope"
else
  target="data/lexicon/${scope}"
fi

rg "$1" "$target"
