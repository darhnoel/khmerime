#!/usr/bin/env python3
"""Disable approved duplicate runtime rows in selected lexicon chunk files."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path


CHUNK_COLUMNS = ["roman", "target", "freq", "freq_lang", "category", "status", "notes"]
DEFAULT_CHUNKS_DIR = Path("data/lexicon/chunks")
DEFAULT_NOTE = "duplicate runtime key; disabled during chunk review"


class DataError(Exception):
    pass


def resolve_chunk_path(chunks_dir: Path, value: str) -> Path:
    path = Path(value)
    if path.exists():
        return path
    if value.isdigit():
        return chunks_dir / f"chunk_{int(value):04}.csv"
    if value.startswith("chunk_") and value.endswith(".csv"):
        return chunks_dir / value
    return chunks_dir / value


def read_chunk(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise DataError(f"{path}: missing header")
        fieldnames = [field.lstrip("\ufeff") if index == 0 else field for index, field in enumerate(reader.fieldnames)]
        if fieldnames != CHUNK_COLUMNS:
            raise DataError(f"{path}: expected columns {','.join(CHUNK_COLUMNS)}, got {','.join(fieldnames)}")
        reader.fieldnames = fieldnames
        rows = list(reader)
    for line_no, row in enumerate(rows, 2):
        if None in row:
            raise DataError(f"{path}:{line_no}: unknown extra columns are not allowed")
    return rows


def write_chunk(path: Path, rows: list[dict[str, str]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=CHUNK_COLUMNS, lineterminator="\n", extrasaction="raise")
        writer.writeheader()
        writer.writerows(rows)


def runtime_key(row: dict[str, str]) -> tuple[str, str, str]:
    return (
        (row.get("roman") or "").strip(),
        (row.get("target") or "").strip(),
        (row.get("freq_lang") or "").strip(),
    )


def is_approved(row: dict[str, str]) -> bool:
    return (row.get("status") or "").strip() == "approved"


def append_note(row: dict[str, str], note: str) -> None:
    current = (row.get("notes") or "").strip()
    if current:
        if note not in current:
            row["notes"] = f"{current}; {note}"
    else:
        row["notes"] = note


def run(args: argparse.Namespace) -> None:
    chunks_dir = Path(args.chunks_dir)
    if not chunks_dir.exists():
        raise DataError(f"{chunks_dir}: chunks directory does not exist")

    chunk_paths = sorted(chunks_dir.glob("*.csv"))
    if not chunk_paths:
        raise DataError(f"{chunks_dir}: no chunk CSV files found")

    drop_paths = [resolve_chunk_path(chunks_dir, value) for value in args.drop_file]
    keep_paths = [resolve_chunk_path(chunks_dir, value) for value in args.keep_file]
    for path in drop_paths + keep_paths:
        if not path.exists():
            raise DataError(f"{path}: file does not exist")

    chunks = {path: read_chunk(path) for path in chunk_paths}
    keep_index: dict[tuple[str, str, str], tuple[Path, int]] = {}
    for path, rows in chunks.items():
        if keep_paths and path not in keep_paths:
            continue
        if path in drop_paths:
            continue
        for line_no, row in enumerate(rows, 2):
            if is_approved(row):
                keep_index.setdefault(runtime_key(row), (path, line_no))

    changes: list[tuple[Path, int, tuple[Path, int], tuple[str, str, str]]] = []
    for path in drop_paths:
        for line_no, row in enumerate(chunks[path], 2):
            if not is_approved(row):
                continue
            key = runtime_key(row)
            existing = keep_index.get(key)
            if existing is None:
                continue
            row["status"] = "disabled"
            append_note(row, args.note)
            changes.append((path, line_no, existing, key))

    for path, line_no, (existing_path, existing_line), (roman, target, freq_lang) in changes:
        print(
            f"{path}:{line_no}: disable duplicate roman={roman!r}, target={target!r}, "
            f"freq_lang={freq_lang!r}; kept {existing_path}:{existing_line}"
        )

    mode = "would disable" if args.dry_run else "disabled"
    print(f"{mode} {len(changes)} rows")

    if not args.dry_run:
        for path in sorted({change[0] for change in changes}):
            write_chunk(path, chunks[path])


def main() -> int:
    parser = argparse.ArgumentParser(description="Disable duplicate approved runtime rows in selected chunk files.")
    parser.add_argument("--chunks-dir", default=str(DEFAULT_CHUNKS_DIR), help="directory containing chunk_*.csv files")
    parser.add_argument(
        "--drop-file",
        action="append",
        required=True,
        help="chunk file to disable duplicates in; accepts 32, chunk_0032.csv, or a path",
    )
    parser.add_argument(
        "--keep-file",
        action="append",
        default=[],
        help="optional chunk file to compare against; if omitted, compares against all other chunks",
    )
    parser.add_argument("--note", default=DEFAULT_NOTE, help="note to append to disabled duplicate rows")
    parser.add_argument("--dry-run", action="store_true", help="report rows without writing files")
    args = parser.parse_args()
    try:
        run(args)
    except DataError as error:
        print(error)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
