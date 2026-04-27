#!/usr/bin/env python3
"""Sort an English-Khmer CSV by the English column.

Default input:
  data/google-10000-english.csv

Default output:
  data/most-common-en-kh.csv
"""

from __future__ import annotations

import argparse
import csv
from pathlib import Path


DEFAULT_INPUT = Path("data/google-10000-english.csv")
DEFAULT_OUTPUT = Path("data/most-common-en-kh.csv")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", default=str(DEFAULT_INPUT), help="Source CSV with English in column 1")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT), help="Destination sorted CSV")
    parser.add_argument(
        "--has-header",
        action="store_true",
        help="Preserve the first row as a header instead of sorting it with data rows",
    )
    return parser.parse_args()


def read_rows(path: Path) -> list[list[str]]:
    with path.open("r", encoding="utf-8", newline="") as handle:
        return [row for row in csv.reader(handle) if row]


def write_rows(path: Path, rows: list[list[str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerows(rows)


def main() -> None:
    args = parse_args()
    input_path = Path(args.input)
    output_path = Path(args.output)

    rows = read_rows(input_path)
    header: list[str] | None = None
    if args.has_header and rows:
        header = rows.pop(0)

    rows.sort(key=lambda row: row[0].casefold())
    if header is not None:
        rows.insert(0, header)

    write_rows(output_path, rows)
    print(f"wrote {len(rows)} rows to {output_path}")


if __name__ == "__main__":
    main()
