#!/usr/bin/env python3
"""Replace roman suffix thomr with thor for lexicon rows targeting Khmer ធម៌ words."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path


CHUNK_COLUMNS = ["roman", "target", "freq", "freq_lang", "category", "status", "notes"]
DEFAULT_CHUNKS_DIR = Path("data/lexicon/chunks")
TARGET_TEXT = "ធម៌"
OLD_SUFFIX = "thomr"
NEW_SUFFIX = "thor"


class DataError(Exception):
    pass


def replacement_roman(roman: str) -> str | None:
    if not roman.endswith(OLD_SUFFIX):
        return None
    return f"{roman[: -len(OLD_SUFFIX)]}{NEW_SUFFIX}"


def load_chunk(path: Path) -> list[dict[str, str]]:
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


def collect_runtime_keys(chunks: dict[Path, list[dict[str, str]]]) -> dict[tuple[str, str, str], tuple[Path, int]]:
    keys: dict[tuple[str, str, str], tuple[Path, int]] = {}
    for path, rows in chunks.items():
        for line_no, row in enumerate(rows, 2):
            if (row.get("status") or "").strip() != "approved":
                continue
            key = (
                (row.get("roman") or "").strip(),
                (row.get("target") or "").strip(),
                (row.get("freq_lang") or "").strip(),
            )
            keys.setdefault(key, (path, line_no))
    return keys


def run(args: argparse.Namespace) -> None:
    chunks_dir = Path(args.chunks_dir)
    if not chunks_dir.exists():
        raise DataError(f"{chunks_dir}: chunks directory does not exist")
    chunk_paths = sorted(chunks_dir.glob("*.csv"))
    if not chunk_paths:
        raise DataError(f"{chunks_dir}: no chunk CSV files found")

    chunks = {path: load_chunk(path) for path in chunk_paths}
    runtime_keys = collect_runtime_keys(chunks)
    changes: list[tuple[Path, int, str, str, str]] = []
    conflicts: list[str] = []

    for path, rows in chunks.items():
        for line_no, row in enumerate(rows, 2):
            target = (row.get("target") or "").strip()
            if TARGET_TEXT not in target:
                continue
            old_roman = (row.get("roman") or "").strip()
            new_roman = replacement_roman(old_roman)
            if new_roman is None:
                continue
            freq_lang = (row.get("freq_lang") or "").strip()
            status = (row.get("status") or "").strip()
            if status == "approved":
                new_key = (new_roman, target, freq_lang)
                existing = runtime_keys.get(new_key)
                if existing is not None and existing != (path, line_no):
                    existing_path, existing_line = existing
                    message = (
                        f"{path}:{line_no}: changing roman {old_roman!r} to {new_roman!r} would duplicate "
                        f"approved runtime key also found at {existing_path}:{existing_line}"
                    )
                    if args.strict:
                        raise DataError(message)
                    conflicts.append(message)
                    continue
                runtime_keys.pop((old_roman, target, freq_lang), None)
                runtime_keys[new_key] = (path, line_no)
            row["roman"] = new_roman
            changes.append((path, line_no, old_roman, new_roman, target))

    for path, line_no, old_roman, new_roman, target in changes:
        print(f"{path}:{line_no}: {old_roman} -> {new_roman} ({target})")

    for conflict in conflicts:
        print(f"SKIP duplicate: {conflict}")

    mode = "would update" if args.dry_run else "updated"
    print(f"{mode} {len(changes)} rows")
    if conflicts:
        print(f"skipped {len(conflicts)} duplicate conflicts")

    if not args.dry_run:
        for path, rows in chunks.items():
            if any(change[0] == path for change in changes):
                write_chunk(path, rows)


def main() -> int:
    parser = argparse.ArgumentParser(description="Replace roman suffix thomr with thor for Khmer ធម៌ chunk rows.")
    parser.add_argument("--chunks-dir", default=str(DEFAULT_CHUNKS_DIR), help="directory containing chunk_*.csv files")
    parser.add_argument("--dry-run", action="store_true", help="report changes without writing files")
    parser.add_argument("--strict", action="store_true", help="fail instead of skipping duplicate runtime keys")
    args = parser.parse_args()
    try:
        run(args)
    except DataError as error:
        print(error)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
