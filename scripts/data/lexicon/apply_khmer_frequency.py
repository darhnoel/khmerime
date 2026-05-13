#!/usr/bin/env python3
"""Apply Khmer target frequency values to lexicon chunk CSV files."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path


CHUNK_COLUMNS = ["roman", "target", "freq", "freq_lang", "category", "status", "notes"]
DEFAULT_CHUNKS_DIR = Path("data/lexicon/chunks")
DEFAULT_FREQUENCY_SOURCE = Path(
    "backup/khmerlang-mobile-keyboard-data/keyboard-data/extracted/mobile-keyboard-data-1gram.csv"
)


class DataError(Exception):
    pass


def is_khmer_char(char: str) -> bool:
    return "\u1780" <= char <= "\u17ff" or "\u19e0" <= char <= "\u19ff"


def is_khmer_target(value: str) -> bool:
    text = value.strip()
    return text != "" and all(is_khmer_char(char) for char in text)


def read_frequency_source(path: Path) -> dict[str, int]:
    frequencies: dict[str, int] = {}
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise DataError(f"{path}: missing header")
        fieldnames = [field.lstrip("\ufeff") if index == 0 else field for index, field in enumerate(reader.fieldnames)]
        if "word" not in fieldnames or "frequency" not in fieldnames:
            raise DataError(f"{path}: expected columns including word,frequency")
        reader.fieldnames = fieldnames
        for line_no, row in enumerate(reader, 2):
            word = (row.get("word") or "").strip()
            raw_frequency = (row.get("frequency") or "").strip()
            if not is_khmer_target(word):
                continue
            try:
                frequency = int(raw_frequency)
            except ValueError as error:
                raise DataError(f"{path}:{line_no}: invalid frequency {raw_frequency!r}") from error
            if frequency <= 0:
                continue
            frequencies[word] = max(frequencies.get(word, 0), frequency)
    return frequencies


def update_chunk(path: Path, frequencies: dict[str, int], dry_run: bool) -> tuple[int, int]:
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise DataError(f"{path}: missing header")
        fieldnames = [field.lstrip("\ufeff") if index == 0 else field for index, field in enumerate(reader.fieldnames)]
        if fieldnames != CHUNK_COLUMNS:
            raise DataError(f"{path}: expected columns {','.join(CHUNK_COLUMNS)}, got {','.join(fieldnames)}")
        reader.fieldnames = fieldnames
        rows = list(reader)

    updated = 0
    matched = 0
    for line_index, row in enumerate(rows, 2):
        if None in row:
            raise DataError(f"{path}:{line_index}: unknown extra columns are not allowed")
        target = (row.get("target") or "").strip()
        freq_lang = (row.get("freq_lang") or "").strip()
        if freq_lang != "km" or not is_khmer_target(target):
            continue
        frequency = frequencies.get(target)
        if frequency is None:
            continue
        matched += 1
        old_frequency = (row.get("freq") or "").strip()
        new_frequency = str(frequency)
        if old_frequency != new_frequency:
            row["freq"] = new_frequency
            updated += 1

    if updated > 0 and not dry_run:
        with path.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=CHUNK_COLUMNS, lineterminator="\n", extrasaction="raise")
            writer.writeheader()
            writer.writerows(rows)

    return matched, updated


def run(args: argparse.Namespace) -> None:
    source = Path(args.source)
    chunks_dir = Path(args.chunks_dir)
    if not source.exists():
        raise DataError(f"{source}: frequency source does not exist")
    if not chunks_dir.exists():
        raise DataError(f"{chunks_dir}: chunks directory does not exist")

    frequencies = read_frequency_source(source)
    if not frequencies:
        raise DataError(f"{source}: no Khmer frequency rows found")

    chunk_paths = sorted(chunks_dir.glob("*.csv"))
    if not chunk_paths:
        raise DataError(f"{chunks_dir}: no chunk CSV files found")

    total_matched = 0
    total_updated = 0
    touched_files = 0
    for path in chunk_paths:
        matched, updated = update_chunk(path, frequencies, args.dry_run)
        total_matched += matched
        total_updated += updated
        if updated:
            touched_files += 1

    mode = "would update" if args.dry_run else "updated"
    print(f"loaded {len(frequencies)} Khmer frequency rows from source CSV")
    print(f"matched {total_matched} km chunk rows")
    print(f"{mode} {total_updated} freq values across {touched_files} chunk files")


def main() -> int:
    parser = argparse.ArgumentParser(description="Apply Khmer target frequency values to lexicon chunks.")
    parser.add_argument("--source", default=str(DEFAULT_FREQUENCY_SOURCE), help="CSV with word,frequency columns")
    parser.add_argument("--chunks-dir", default=str(DEFAULT_CHUNKS_DIR), help="directory containing chunk_*.csv files")
    parser.add_argument("--dry-run", action="store_true", help="report changes without writing files")
    args = parser.parse_args()
    try:
        run(args)
    except DataError as error:
        print(error)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
