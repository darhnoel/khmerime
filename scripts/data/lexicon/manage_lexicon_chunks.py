#!/usr/bin/env python3
"""Manage human-reviewable KhmerIME lexicon chunks."""

from __future__ import annotations

import argparse
import csv
import io
import sys
from dataclasses import dataclass
from pathlib import Path


CHUNK_COLUMNS = ["roman", "target", "freq", "freq_lang", "category", "status", "notes"]
RUNTIME_COLUMNS = ["roman", "target", "freq", "freq_lang"]
VALID_FREQ_LANGS = {"km", "en", "ja", "zh", "ko"}
VALID_CATEGORIES = {"unclassified", "words", "names", "places", "phrases", "common"}
VALID_STATUSES = {"approved", "draft", "rejected", "disabled"}
DEFAULT_CHUNKS_DIR = Path("data/lexicon/chunks")
DEFAULT_RUNTIME_PATH = Path("data/roman_lookup.csv")


@dataclass(frozen=True)
class SourceLocation:
    path: Path
    line_no: int


@dataclass(frozen=True)
class ChunkRow:
    roman: str
    target: str
    freq: int
    freq_lang: str
    category: str
    status: str
    notes: str
    location: SourceLocation


class DataError(Exception):
    pass


def normalize_roman(value: str) -> str:
    output = []
    for char in value.lower():
        codepoint = ord(char)
        if (
            char.isascii()
            and (char.isalnum() or char in {"_", ",", " "})
            or 0x00C0 <= codepoint <= 0x00FF
            or 0x0621 <= codepoint <= 0x064A
            or 0x0660 <= codepoint <= 0x0669
            or 0x1780 <= codepoint <= 0x17D2
        ):
            output.append(char)
    return "".join(output)


def parse_freq(raw: str, location: SourceLocation, allow_blank: bool) -> int:
    value = raw.strip()
    if value == "":
        if allow_blank:
            return 1
        raise DataError(f"{location.path}:{location.line_no}: missing freq")
    try:
        parsed = int(value)
    except ValueError as error:
        raise DataError(f"{location.path}:{location.line_no}: invalid freq {value!r}; expected positive integer") from error
    if parsed <= 0:
        raise DataError(f"{location.path}:{location.line_no}: invalid freq {value!r}; expected positive integer")
    return parsed


def read_runtime_rows(path: Path) -> list[tuple[str, str, int, str]]:
    rows: list[tuple[str, str, int, str]] = []
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.reader(handle)
        first_data_row = True
        for line_no, row in enumerate(reader, 1):
            if not row or all(not field.strip() for field in row):
                continue
            if line_no == 1 and row:
                row[0] = row[0].lstrip("\ufeff")
            lowered = [field.strip().lower() for field in row]
            if first_data_row and lowered in (
                ["roman", "target"],
                ["roman", "target", "freq"],
                ["roman", "target", "freq", "freq_lang"],
            ):
                first_data_row = False
                continue
            first_data_row = False
            if len(row) not in {2, 3, 4}:
                raise DataError(f"{path}:{line_no}: expected 2, 3, or 4 runtime columns, got {len(row)}")
            roman = row[0].strip()
            target = row[1].strip()
            if roman == "" or target == "":
                raise DataError(f"{path}:{line_no}: roman and target are required")
            freq = parse_freq(row[2] if len(row) >= 3 else "", SourceLocation(path, line_no), allow_blank=True)
            freq_lang = row[3].strip() if len(row) >= 4 else "km"
            if freq_lang == "":
                freq_lang = "km"
            if freq_lang not in VALID_FREQ_LANGS:
                raise DataError(f"{path}:{line_no}: invalid freq_lang {freq_lang!r}")
            rows.append((roman, target, freq, freq_lang))
    return rows


def write_csv(path: Path, rows: list[dict[str, str]], columns: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns, lineterminator="\n", extrasaction="raise")
        writer.writeheader()
        writer.writerows(rows)


def read_chunk_dicts(path: Path) -> list[tuple[dict[str, str], int]]:
    rows: list[tuple[dict[str, str], int]] = []
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise DataError(f"{path}: missing header")
        fieldnames = [field.lstrip("\ufeff") if index == 0 else field for index, field in enumerate(reader.fieldnames)]
        if fieldnames != CHUNK_COLUMNS:
            raise DataError(f"{path}: expected columns {','.join(CHUNK_COLUMNS)}, got {','.join(fieldnames)}")
        reader.fieldnames = fieldnames
        for row_index, raw_row in enumerate(reader, 2):
            if raw_row is None:
                continue
            if None in raw_row:
                raise DataError(f"{path}:{row_index}: unknown extra columns are not allowed")
            rows.append(({column: raw_row.get(column, "") or "" for column in CHUNK_COLUMNS}, row_index))
    return rows


def chunk_row_from_dict(raw_row: dict[str, str], location: SourceLocation) -> ChunkRow:
    roman = (raw_row.get("roman") or "").strip()
    target = (raw_row.get("target") or "").strip()
    if roman == "":
        raise DataError(f"{location.path}:{location.line_no}: roman is required")
    if target == "":
        raise DataError(f"{location.path}:{location.line_no}: target is required")
    freq = parse_freq(raw_row.get("freq") or "", location, allow_blank=True)
    freq_lang = (raw_row.get("freq_lang") or "").strip()
    if freq_lang == "":
        raise DataError(f"{location.path}:{location.line_no}: freq_lang is required")
    if freq_lang not in VALID_FREQ_LANGS:
        raise DataError(f"{location.path}:{location.line_no}: invalid freq_lang {freq_lang!r}")
    category = (raw_row.get("category") or "").strip()
    if category not in VALID_CATEGORIES:
        raise DataError(f"{location.path}:{location.line_no}: invalid category {category!r}")
    status = (raw_row.get("status") or "").strip()
    if status not in VALID_STATUSES:
        raise DataError(f"{location.path}:{location.line_no}: invalid status {status!r}")
    notes = raw_row.get("notes") or ""
    return ChunkRow(roman, target, freq, freq_lang, category, status, notes, location)


def split_runtime(args: argparse.Namespace) -> None:
    runtime_path = Path(args.runtime)
    chunks_dir = Path(args.chunks_dir)
    chunk_size = args.chunk_size
    if chunk_size <= 0:
        raise DataError("--chunk-size must be positive")
    if chunks_dir.exists() and any(chunks_dir.glob("*.csv")) and not args.force:
        raise DataError(f"{chunks_dir} already contains CSV files; pass --force to replace them")
    chunks_dir.mkdir(parents=True, exist_ok=True)
    if args.force:
        for path in chunks_dir.glob("*.csv"):
            path.unlink()

    rows = read_runtime_rows(runtime_path)
    seen_runtime_keys: set[tuple[str, str, str]] = set()
    output_rows: list[dict[str, str]] = []
    disabled_duplicates = 0
    for roman, target, freq, freq_lang in rows:
        key = (roman, target, freq_lang)
        status = "approved"
        notes = ""
        if key in seen_runtime_keys:
            status = "disabled"
            notes = "duplicate runtime key from initial split; review before re-enabling"
            disabled_duplicates += 1
        else:
            seen_runtime_keys.add(key)
        output_rows.append(
            {
                "roman": roman,
                "target": target,
                "freq": str(freq),
                "freq_lang": freq_lang,
                "category": "unclassified",
                "status": status,
                "notes": notes,
            }
        )

    for chunk_index, start in enumerate(range(0, len(output_rows), chunk_size), 1):
        chunk_rows = output_rows[start : start + chunk_size]
        chunk_path = chunks_dir / f"chunk_{chunk_index:04}.csv"
        write_csv(chunk_path, chunk_rows, CHUNK_COLUMNS)

    print(
        f"wrote {len(output_rows)} rows to {chunks_dir} in {(len(output_rows) + chunk_size - 1) // chunk_size} chunks"
    )
    if disabled_duplicates:
        print(f"marked {disabled_duplicates} duplicate runtime rows as disabled")


def read_chunk_rows(chunks_dir: Path) -> list[ChunkRow]:
    rows: list[ChunkRow] = []
    chunk_paths = sorted(chunks_dir.glob("*.csv"))
    if not chunk_paths:
        raise DataError(f"no chunk CSV files found under {chunks_dir}")

    for path in chunk_paths:
        for raw_row, row_index in read_chunk_dicts(path):
            rows.append(chunk_row_from_dict(raw_row, SourceLocation(path, row_index)))
    return rows


def validate_rows(rows: list[ChunkRow]) -> None:
    runtime_keys: dict[tuple[str, str, str], ChunkRow] = {}
    target_freqs: dict[tuple[str, str], ChunkRow] = {}
    for row in rows:
        if row.status != "approved":
            continue
        runtime_key = (row.roman, row.target, row.freq_lang)
        existing_runtime = runtime_keys.get(runtime_key)
        if existing_runtime is not None:
            raise DataError(
                f'ERROR {row.location.path}:{row.location.line_no}\n'
                f"Duplicate approved runtime entry also found in "
                f"{existing_runtime.location.path}:{existing_runtime.location.line_no}:\n"
                f'roman="{row.roman}", target="{row.target}", freq_lang="{row.freq_lang}"\n\n'
                "Please resolve manually by keeping one row and choosing the intended freq/category/notes."
            )
        runtime_keys[runtime_key] = row

        target_key = (row.target, row.freq_lang)
        existing_target = target_freqs.get(target_key)
        if existing_target is not None and existing_target.freq != row.freq:
            raise DataError(
                f'ERROR {row.location.path}:{row.location.line_no}\n'
                f'Inconsistent frequency for target="{row.target}", freq_lang="{row.freq_lang}".\n\n'
                "Existing:\n"
                f"  {existing_target.location.path}:{existing_target.location.line_no}\n"
                f'  roman="{existing_target.roman}", freq={existing_target.freq}\n\n'
                "Current:\n"
                f"  {row.location.path}:{row.location.line_no}\n"
                f'  roman="{row.roman}", freq={row.freq}\n\n'
                "Rows with the same target and freq_lang should use one shared frequency.\n"
                "Please choose one intended freq."
            )
        target_freqs[target_key] = row


def generated_runtime_text(rows: list[ChunkRow]) -> str:
    approved = [row for row in rows if row.status == "approved"]
    approved.sort(key=lambda row: (normalize_roman(row.roman), row.target, row.freq_lang, row.roman))
    output = io.StringIO()
    writer = csv.DictWriter(output, fieldnames=RUNTIME_COLUMNS, lineterminator="\n")
    writer.writeheader()
    for row in approved:
        writer.writerow(
            {
                "roman": row.roman,
                "target": row.target,
                "freq": str(row.freq),
                "freq_lang": row.freq_lang,
            }
        )
    return output.getvalue()


def build_runtime(args: argparse.Namespace) -> None:
    chunks_dir = Path(args.chunks_dir)
    runtime_path = Path(args.runtime)
    rows = read_chunk_rows(chunks_dir)
    validate_rows(rows)
    runtime_path.parent.mkdir(parents=True, exist_ok=True)
    runtime_path.write_text(generated_runtime_text(rows), encoding="utf-8")
    print(f"wrote generated runtime lexicon to {runtime_path}")


def check_data(args: argparse.Namespace) -> None:
    chunks_dir = Path(args.chunks_dir)
    runtime_path = Path(args.runtime)
    rows = read_chunk_rows(chunks_dir)
    validate_rows(rows)
    expected = generated_runtime_text(rows)
    try:
        current = runtime_path.read_text(encoding="utf-8")
    except FileNotFoundError as error:
        raise DataError(f"{runtime_path}: generated runtime lexicon is missing; run make data-build") from error
    if current != expected:
        raise DataError(f"{runtime_path}: generated runtime lexicon is stale; run make data-build")
    print(f"validated {len(rows)} source rows from {chunks_dir}")
    print(f"{runtime_path} is up to date")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    split = subparsers.add_parser("split", help="split runtime lexicon into reviewable chunks")
    split.add_argument("--runtime", default=str(DEFAULT_RUNTIME_PATH))
    split.add_argument("--chunks-dir", default=str(DEFAULT_CHUNKS_DIR))
    split.add_argument("--chunk-size", type=int, default=1000)
    split.add_argument("--force", action="store_true")
    split.set_defaults(func=split_runtime)

    build = subparsers.add_parser("build", help="generate runtime lexicon from chunks")
    build.add_argument("--runtime", default=str(DEFAULT_RUNTIME_PATH))
    build.add_argument("--chunks-dir", default=str(DEFAULT_CHUNKS_DIR))
    build.set_defaults(func=build_runtime)

    check = subparsers.add_parser("check", help="validate chunks and generated runtime lexicon")
    check.add_argument("--runtime", default=str(DEFAULT_RUNTIME_PATH))
    check.add_argument("--chunks-dir", default=str(DEFAULT_CHUNKS_DIR))
    check.set_defaults(func=check_data)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        args.func(args)
    except DataError as error:
        print(error, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
