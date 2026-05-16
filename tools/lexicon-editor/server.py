#!/usr/bin/env python3
"""Local spreadsheet editor for KhmerIME lexicon chunks."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import mimetypes
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import webbrowser
from copy import deepcopy
from dataclasses import dataclass, field
from datetime import date, datetime
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse


ROOT = Path(__file__).resolve().parents[2]
STATIC_DIR = Path(__file__).resolve().parent / "static"
MANAGER_DIR = ROOT / "scripts" / "data" / "lexicon"
if str(MANAGER_DIR) not in sys.path:
    sys.path.insert(0, str(MANAGER_DIR))

import manage_lexicon_chunks as chunks  # noqa: E402


Row = dict[str, str]


class EditorError(Exception):
    def __init__(self, message: str, status: int = HTTPStatus.BAD_REQUEST):
        super().__init__(message)
        self.status = status


@dataclass
class FileStamp:
    mtime_ns: int
    size: int


@dataclass
class ChunkDraft:
    name: str
    path: Path
    stamp: FileStamp
    rows: list[Row]
    originals: dict[str, Row]


@dataclass
class EditorState:
    root: Path = ROOT
    chunks_dir: Path = ROOT / "data" / "lexicon" / "chunks"
    runtime_path: Path = ROOT / "data" / "roman_lookup.csv"
    lock: threading.RLock = field(default_factory=threading.RLock)
    chunk_paths: list[Path] = field(default_factory=list)
    stamps: dict[str, FileStamp] = field(default_factory=dict)
    drafts: dict[str, ChunkDraft] = field(default_factory=dict)
    dirty_chunks: set[str] = field(default_factory=set)
    undo_stack: list[dict[str, object]] = field(default_factory=list)
    redo_stack: list[dict[str, object]] = field(default_factory=list)
    new_counter: int = 0
    last_diff: str = ""

    def __post_init__(self) -> None:
        self.scan_chunks()

    def scan_chunks(self) -> None:
        self.chunk_paths = sorted(self.chunks_dir.glob("*.csv"))
        self.stamps = {path.name: self.file_stamp(path) for path in self.chunk_paths}

    def file_stamp(self, path: Path) -> FileStamp:
        stat = path.stat()
        return FileStamp(mtime_ns=stat.st_mtime_ns, size=stat.st_size)

    def chunk_names(self) -> list[str]:
        return [path.name for path in self.chunk_paths]

    def chunk_path(self, name: str) -> Path:
        path = self.chunks_dir / name
        if path not in self.chunk_paths and not path.exists():
            raise EditorError(f"unknown chunk {name!r}")
        if path.parent.resolve() != self.chunks_dir.resolve() or path.name != name:
            raise EditorError(f"invalid chunk {name!r}")
        return path

    def row_id(self, chunk_name: str, index: int, raw: Row, line_no: int) -> str:
        digest_source = "\x1f".join(raw.get(column, "") for column in chunks.CHUNK_COLUMNS)
        digest = hashlib.sha1(digest_source.encode("utf-8")).hexdigest()[:10]
        return f"{chunk_name}:{index}:{line_no}:{digest}"

    def make_row(self, chunk_name: str, index: int, raw: Row, line_no: int | None) -> Row:
        row = {column: raw.get(column, "") or "" for column in chunks.CHUNK_COLUMNS}
        row["_id"] = self.row_id(chunk_name, index, row, line_no or 0) if line_no is not None else self.next_new_id()
        row["_chunk"] = chunk_name
        row["_orig_line"] = "" if line_no is None else str(line_no)
        return row

    def next_new_id(self) -> str:
        self.new_counter += 1
        return f"new:{self.new_counter}"

    def read_chunk_rows(self, path: Path) -> list[Row]:
        rows: list[Row] = []
        for index, (raw, line_no) in enumerate(chunks.read_chunk_dicts(path), 1):
            rows.append(self.make_row(path.name, index, raw, line_no))
        return rows

    def ensure_draft(self, chunk_name: str) -> ChunkDraft:
        draft = self.drafts.get(chunk_name)
        if draft is not None:
            return draft
        path = self.chunk_path(chunk_name)
        rows = self.read_chunk_rows(path)
        originals = {row["_id"]: deepcopy(row) for row in rows}
        draft = ChunkDraft(chunk_name, path, self.file_stamp(path), rows, originals)
        self.drafts[chunk_name] = draft
        return draft

    def rows_for_chunk(self, path: Path) -> list[Row]:
        draft = self.drafts.get(path.name)
        if draft is not None and path.name in self.dirty_chunks:
            return deepcopy(draft.rows)
        if draft is not None:
            return deepcopy(draft.rows)
        return self.read_chunk_rows(path)

    def current_rows(self) -> list[Row]:
        rows: list[Row] = []
        for path in self.chunk_paths:
            rows.extend(self.rows_for_chunk(path))
        return rows

    def snapshot(self) -> dict[str, object]:
        return {
            "drafts": deepcopy(self.drafts),
            "dirty_chunks": set(self.dirty_chunks),
            "new_counter": self.new_counter,
        }

    def restore_snapshot(self, snapshot: dict[str, object]) -> None:
        self.drafts = deepcopy(snapshot["drafts"])  # type: ignore[assignment]
        self.dirty_chunks = set(snapshot["dirty_chunks"])  # type: ignore[arg-type]
        self.new_counter = int(snapshot["new_counter"])

    def push_undo(self) -> None:
        self.undo_stack.append(self.snapshot())
        if len(self.undo_stack) > 100:
            self.undo_stack.pop(0)
        self.redo_stack.clear()

    def mark_dirty(self, chunk_name: str) -> None:
        self.dirty_chunks.add(chunk_name)

    def find_row(self, row_id: str) -> tuple[ChunkDraft, int, Row]:
        chunk_name = row_id.split(":", 1)[0]
        if chunk_name == "new":
            for draft in self.drafts.values():
                for index, row in enumerate(draft.rows):
                    if row["_id"] == row_id:
                        return draft, index, row
            raise EditorError(f"unknown row {row_id!r}")
        draft = self.ensure_draft(chunk_name)
        for index, row in enumerate(draft.rows):
            if row["_id"] == row_id:
                return draft, index, row
        raise EditorError(f"unknown row {row_id!r}")

    def row_errors(self, row: Row) -> list[str]:
        errors: list[str] = []
        if not row.get("roman", "").strip():
            errors.append("roman is required")
        if not row.get("target", "").strip():
            errors.append("target is required")
        freq = row.get("freq", "").strip()
        try:
            parsed_freq = int(freq or "1")
            if parsed_freq <= 0:
                errors.append("freq must be a positive integer")
        except ValueError:
            errors.append("freq must be a positive integer")
        if row.get("freq_lang", "").strip() not in chunks.VALID_FREQ_LANGS:
            errors.append("invalid freq_lang")
        if row.get("category", "").strip() not in chunks.VALID_CATEGORIES:
            errors.append("invalid category")
        if row.get("status", "").strip() not in chunks.VALID_STATUSES:
            errors.append("invalid status")
        return errors

    def row_to_public(self, row: Row, position: int, problem_map: dict[str, list[str]] | None = None) -> dict[str, object]:
        errors = self.row_errors(row)
        warnings = [] if problem_map is None else problem_map.get(row["_id"], [])
        draft = self.drafts.get(row["_chunk"])
        original = None if draft is None else draft.originals.get(row["_id"])
        is_dirty = False
        if draft is not None and row["_chunk"] in self.dirty_chunks:
            is_dirty = original is None or any(row.get(column, "") != original.get(column, "") for column in chunks.CHUNK_COLUMNS)
        visible = {column: row.get(column, "") for column in chunks.CHUNK_COLUMNS}
        visible.update(
            {
                "id": row["_id"],
                "chunk": row["_chunk"],
                "row": position,
                "orig_line": row.get("_orig_line", ""),
                "runtime": "included" if row.get("status") == "approved" else "excluded",
                "dirty": is_dirty,
                "errors": errors,
                "warnings": warnings,
                "valid": not errors,
            }
        )
        return visible

    def build_problem_map(self, rows: list[Row]) -> dict[str, list[str]]:
        warnings: dict[str, list[str]] = {}
        runtime_keys: dict[tuple[str, str, str], list[Row]] = {}
        target_freqs: dict[tuple[str, str], dict[str, list[Row]]] = {}
        for row in rows:
            if row.get("status") == "approved":
                runtime_keys.setdefault((row.get("roman", ""), row.get("target", ""), row.get("freq_lang", "")), []).append(row)
                target_freqs.setdefault((row.get("target", ""), row.get("freq_lang", "")), {}).setdefault(row.get("freq", ""), []).append(row)
        for group in runtime_keys.values():
            if len(group) > 1:
                for row in group:
                    warnings.setdefault(row["_id"], []).append("duplicate approved runtime key")
        for freq_groups in target_freqs.values():
            if len(freq_groups) > 1:
                for group in freq_groups.values():
                    for row in group:
                        warnings.setdefault(row["_id"], []).append("inconsistent target frequency")
        for row in rows:
            if row.get("status") in {"disabled", "rejected"}:
                warnings.setdefault(row["_id"], []).append(f"status is {row.get('status')}")
        return warnings

    def filtered_rows(self, params: dict[str, str]) -> tuple[list[Row], dict[str, list[str]]]:
        query = params.get("query", "").strip().lower()
        chunk_filter = params.get("chunk", "").strip()
        status_filter = params.get("status", "").strip()
        category_filter = params.get("category", "").strip()
        all_rows = self.current_rows()
        problem_map = self.build_problem_map(all_rows)
        output: list[Row] = []
        for row in all_rows:
            if chunk_filter and row.get("_chunk") != chunk_filter:
                continue
            if status_filter and row.get("status") != status_filter:
                continue
            if category_filter and row.get("category") != category_filter:
                continue
            if query:
                haystack = " ".join([row.get("roman", ""), row.get("target", ""), row.get("notes", "")]).lower()
                if query not in haystack:
                    continue
            output.append(row)
        return output, problem_map

    def api_meta(self) -> dict[str, object]:
        external_changes = []
        for name in self.dirty_chunks:
            draft = self.drafts[name]
            if self.file_stamp(draft.path) != draft.stamp:
                external_changes.append(name)
        return {
            "chunks": self.chunk_names(),
            "columns": chunks.CHUNK_COLUMNS,
            "freq_langs": sorted(chunks.VALID_FREQ_LANGS),
            "categories": sorted(chunks.VALID_CATEGORIES),
            "statuses": sorted(chunks.VALID_STATUSES),
            "dirty_chunks": sorted(self.dirty_chunks),
            "dirty_count": len(self.dirty_chunks),
            "can_undo": bool(self.undo_stack),
            "can_redo": bool(self.redo_stack),
            "external_changes": external_changes,
        }

    def api_rows(self, query: dict[str, list[str]]) -> dict[str, object]:
        params = {key: values[-1] for key, values in query.items() if values}
        page = max(1, int(params.get("page", "1") or "1"))
        page_size = min(250, max(1, int(params.get("page_size", "100") or "100")))
        rows, problem_map = self.filtered_rows(params)
        start = (page - 1) * page_size
        page_rows = rows[start : start + page_size]
        last_page = max(1, (len(rows) + page_size - 1) // page_size)
        positions: dict[str, int] = {}
        for path in self.chunk_paths:
            for index, row in enumerate(self.rows_for_chunk(path), 1):
                positions[row["_id"]] = index
        return {
            "data": [self.row_to_public(row, positions.get(row["_id"], 0), problem_map) for row in page_rows],
            "last_page": last_page,
            "total": len(rows),
        }

    def api_edit_cell(self, payload: dict[str, object]) -> dict[str, object]:
        row_id = str(payload.get("row_id", ""))
        column = str(payload.get("column", ""))
        value = str(payload.get("value", ""))
        if column not in chunks.CHUNK_COLUMNS:
            raise EditorError(f"cannot edit column {column!r}")
        draft, _, row = self.find_row(row_id)
        if row.get(column, "") == value:
            return {"row": self.row_to_public(row, draft.rows.index(row) + 1)}
        self.push_undo()
        row[column] = value
        self.mark_dirty(draft.name)
        return {"row": self.row_to_public(row, draft.rows.index(row) + 1), "meta": self.api_meta()}

    def api_add_row(self, payload: dict[str, object]) -> dict[str, object]:
        after_row_id = str(payload.get("after_row_id") or "")
        chunk_name = str(payload.get("chunk") or "")
        insert_index = 0
        if after_row_id:
            draft, index, _ = self.find_row(after_row_id)
            chunk_name = draft.name
            insert_index = index + 1
        if not chunk_name:
            raise EditorError("select a chunk before adding a row")
        draft = self.ensure_draft(chunk_name)
        self.push_undo()
        row = self.make_row(
            chunk_name,
            len(draft.rows) + 1,
            {
                "roman": str(payload.get("roman") or ""),
                "target": str(payload.get("target") or ""),
                "freq": str(payload.get("freq") or "1"),
                "freq_lang": str(payload.get("freq_lang") or "km"),
                "category": str(payload.get("category") or "unclassified"),
                "status": str(payload.get("status") or "approved"),
                "notes": str(payload.get("notes") or ""),
            },
            None,
        )
        draft.rows.insert(insert_index, row)
        self.mark_dirty(draft.name)
        return {"row": self.row_to_public(row, insert_index + 1), "meta": self.api_meta()}

    def append_disable_note(self, notes: str) -> str:
        note = f"disabled in lexicon editor {date.today().isoformat()}"
        if note in notes:
            return notes
        return f"{notes}; {note}" if notes.strip() else note

    def api_soft_remove(self, payload: dict[str, object]) -> dict[str, object]:
        row_ids = [str(value) for value in payload.get("row_ids", [])]
        if not row_ids:
            raise EditorError("no rows selected")
        self.push_undo()
        touched: set[str] = set()
        for row_id in row_ids:
            draft, _, row = self.find_row(row_id)
            if row.get("status") == "rejected":
                continue
            if row.get("status") != "disabled":
                row["status"] = "disabled"
                row["notes"] = self.append_disable_note(row.get("notes", ""))
                touched.add(draft.name)
        for name in touched:
            self.mark_dirty(name)
        return {"updated": len(row_ids), "meta": self.api_meta()}

    def api_bulk_edit(self, payload: dict[str, object]) -> dict[str, object]:
        row_ids = [str(value) for value in payload.get("row_ids", [])]
        column = str(payload.get("column", ""))
        value = str(payload.get("value", ""))
        allowed = {
            "category": chunks.VALID_CATEGORIES,
            "status": chunks.VALID_STATUSES,
            "freq_lang": chunks.VALID_FREQ_LANGS,
        }
        if column not in allowed:
            raise EditorError("bulk edit supports category, status, or freq_lang only")
        if value not in allowed[column]:
            raise EditorError(f"invalid {column} value {value!r}")
        if not row_ids:
            raise EditorError("no rows selected")
        self.push_undo()
        touched: set[str] = set()
        for row_id in row_ids:
            draft, _, row = self.find_row(row_id)
            row[column] = value
            touched.add(draft.name)
        for name in touched:
            self.mark_dirty(name)
        return {"updated": len(row_ids), "meta": self.api_meta()}

    def api_move_rows(self, payload: dict[str, object]) -> dict[str, object]:
        row_ids = [str(value) for value in payload.get("row_ids", [])]
        direction = str(payload.get("direction", ""))
        if direction not in {"up", "down", "top", "bottom"}:
            raise EditorError("invalid movement direction")
        if not row_ids:
            raise EditorError("no rows selected")
        located = [self.find_row(row_id) for row_id in row_ids]
        chunk_names = {draft.name for draft, _, _ in located}
        if len(chunk_names) != 1:
            raise EditorError("row movement is limited to one chunk")
        draft = located[0][0]
        selected = {row["_id"] for _, _, row in located}
        self.push_undo()
        if direction == "up":
            for index in range(1, len(draft.rows)):
                if draft.rows[index]["_id"] in selected and draft.rows[index - 1]["_id"] not in selected:
                    draft.rows[index - 1], draft.rows[index] = draft.rows[index], draft.rows[index - 1]
        elif direction == "down":
            for index in range(len(draft.rows) - 2, -1, -1):
                if draft.rows[index]["_id"] in selected and draft.rows[index + 1]["_id"] not in selected:
                    draft.rows[index + 1], draft.rows[index] = draft.rows[index], draft.rows[index + 1]
        else:
            moving = [row for row in draft.rows if row["_id"] in selected]
            remaining = [row for row in draft.rows if row["_id"] not in selected]
            draft.rows = moving + remaining if direction == "top" else remaining + moving
        self.mark_dirty(draft.name)
        return {"moved": len(row_ids), "meta": self.api_meta()}

    def api_revert_row(self, payload: dict[str, object]) -> dict[str, object]:
        row_id = str(payload.get("row_id", ""))
        draft, index, _ = self.find_row(row_id)
        self.push_undo()
        original = draft.originals.get(row_id)
        if original is None:
            draft.rows.pop(index)
        else:
            draft.rows[index] = deepcopy(original)
        self.mark_dirty(draft.name)
        return {"meta": self.api_meta()}

    def api_undo(self) -> dict[str, object]:
        if not self.undo_stack:
            return {"meta": self.api_meta()}
        self.redo_stack.append(self.snapshot())
        self.restore_snapshot(self.undo_stack.pop())
        return {"meta": self.api_meta()}

    def api_redo(self) -> dict[str, object]:
        if not self.redo_stack:
            return {"meta": self.api_meta()}
        self.undo_stack.append(self.snapshot())
        self.restore_snapshot(self.redo_stack.pop())
        return {"meta": self.api_meta()}

    def api_discard_draft(self) -> dict[str, object]:
        self.drafts.clear()
        self.dirty_chunks.clear()
        self.undo_stack.clear()
        self.redo_stack.clear()
        return {"meta": self.api_meta()}

    def api_reload(self, payload: dict[str, object]) -> dict[str, object]:
        force = bool(payload.get("force"))
        if self.dirty_chunks and not force:
            return {"needs_confirmation": True, "dirty_chunks": sorted(self.dirty_chunks)}
        self.drafts.clear()
        self.dirty_chunks.clear()
        self.undo_stack.clear()
        self.redo_stack.clear()
        self.scan_chunks()
        return {"needs_confirmation": False, "meta": self.api_meta()}

    def api_problems(self) -> dict[str, object]:
        rows = self.current_rows()
        problem_map = self.build_problem_map(rows)
        problems: list[dict[str, object]] = []
        positions: dict[str, int] = {}
        for path in self.chunk_paths:
            for index, row in enumerate(self.rows_for_chunk(path), 1):
                positions[row["_id"]] = index
        for row in rows:
            for error in self.row_errors(row):
                problems.append({"type": "invalid row", "message": error, "row": self.row_to_public(row, positions.get(row["_id"], 0))})
            for warning in problem_map.get(row["_id"], []):
                problems.append({"type": warning, "message": warning, "row": self.row_to_public(row, positions.get(row["_id"], 0), problem_map)})
        priority = {
            "invalid row": 0,
            "duplicate approved runtime key": 1,
            "inconsistent target frequency": 2,
            "status is disabled": 3,
            "status is rejected": 4,
        }
        problems.sort(
            key=lambda item: (
                priority.get(str(item["type"]), 99),
                str(item["row"]["chunk"]),  # type: ignore[index]
                int(item["row"]["row"]),  # type: ignore[index]
            )
        )
        return {"problems": problems[:1000], "total": len(problems)}

    def strict_chunk_rows_from_current(self) -> list[chunks.ChunkRow]:
        strict_rows: list[chunks.ChunkRow] = []
        for path in self.chunk_paths:
            for index, row in enumerate(self.rows_for_chunk(path), 2):
                strict_rows.append(chunks.chunk_row_from_dict(row, chunks.SourceLocation(path, index)))
        return strict_rows

    def validate_all_for_save(self) -> None:
        errors = []
        positions = {}
        for path in self.chunk_paths:
            for index, row in enumerate(self.rows_for_chunk(path), 1):
                positions[row["_id"]] = index
        for row in self.current_rows():
            row_errors = self.row_errors(row)
            if row_errors:
                display_row = positions.get(row["_id"], "?")
                original = row.get("_orig_line") or "new"
                errors.append(
                    f"{row['_chunk']} row {display_row} (orig {original}, id {row['_id']}): {', '.join(row_errors)}"
                )
        if errors:
            raise EditorError("cannot save while rows are invalid:\n" + "\n".join(errors[:30]))
        chunks.validate_rows(self.strict_chunk_rows_from_current())

    def ensure_no_external_dirty_changes(self) -> None:
        changed = []
        for name in sorted(self.dirty_chunks):
            draft = self.drafts[name]
            if self.file_stamp(draft.path) != draft.stamp:
                changed.append(name)
        if changed:
            raise EditorError("dirty chunk changed on disk; reload before saving: " + ", ".join(changed), HTTPStatus.CONFLICT)

    def backup_dirty_chunks(self) -> Path:
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        backup_dir = self.root / ".lexicon-editor" / "backups" / stamp
        backup_dir.mkdir(parents=True, exist_ok=True)
        for name in sorted(self.dirty_chunks):
            source = self.drafts[name].path
            shutil.copy2(source, backup_dir / source.name)
        return backup_dir

    def write_dirty_chunks(self) -> None:
        for name in sorted(self.dirty_chunks):
            draft = self.drafts[name]
            output_rows = [{column: row.get(column, "") for column in chunks.CHUNK_COLUMNS} for row in draft.rows]
            fd, temp_name = tempfile.mkstemp(prefix=f".{draft.path.name}.", suffix=".tmp", dir=draft.path.parent)
            os.close(fd)
            temp_path = Path(temp_name)
            try:
                chunks.write_csv(temp_path, output_rows, chunks.CHUNK_COLUMNS)
                for line_index, (raw, _) in enumerate(chunks.read_chunk_dicts(temp_path), 2):
                    chunks.chunk_row_from_dict(raw, chunks.SourceLocation(temp_path, line_index))
                os.replace(temp_path, draft.path)
            finally:
                if temp_path.exists():
                    temp_path.unlink()

    def build_runtime_from_disk(self) -> None:
        rows = chunks.read_chunk_rows(self.chunks_dir)
        chunks.validate_rows(rows)
        self.runtime_path.write_text(chunks.generated_runtime_text(rows), encoding="utf-8")
        expected = chunks.generated_runtime_text(chunks.read_chunk_rows(self.chunks_dir))
        current = self.runtime_path.read_text(encoding="utf-8")
        if current != expected:
            raise EditorError("generated runtime lexicon is stale after build")

    def git_diff(self) -> str:
        result = subprocess.run(
            ["git", "diff", "--", "data/lexicon/chunks", "data/roman_lookup.csv"],
            cwd=self.root,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        return result.stdout

    def api_save_build_check(self) -> dict[str, object]:
        if not self.dirty_chunks:
            self.build_runtime_from_disk()
            self.last_diff = self.git_diff()
            return {"message": "checked existing chunks and runtime CSV", "diff": self.last_diff, "meta": self.api_meta()}
        self.validate_all_for_save()
        self.ensure_no_external_dirty_changes()
        backup_dir = self.backup_dirty_chunks()
        touched = sorted(self.dirty_chunks)
        self.write_dirty_chunks()
        self.build_runtime_from_disk()
        self.scan_chunks()
        for name in touched:
            self.drafts.pop(name, None)
        self.dirty_chunks.clear()
        self.undo_stack.clear()
        self.redo_stack.clear()
        self.last_diff = self.git_diff()
        return {
            "message": "saved, built, and checked",
            "backup_dir": str(backup_dir.relative_to(self.root)),
            "diff": self.last_diff,
            "meta": self.api_meta(),
        }


STATE = EditorState()


class Handler(BaseHTTPRequestHandler):
    server_version = "KhmerImeLexiconEditor/0.1"

    def log_message(self, fmt: str, *args: object) -> None:
        sys.stderr.write("%s - %s\n" % (self.address_string(), fmt % args))

    def send_json(self, payload: object, status: int = HTTPStatus.OK) -> None:
        body = json.dumps(payload, ensure_ascii=False, indent=2).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def read_json(self) -> dict[str, object]:
        length = int(self.headers.get("Content-Length", "0") or "0")
        if length == 0:
            return {}
        raw = self.rfile.read(length)
        try:
            data = json.loads(raw.decode("utf-8"))
        except json.JSONDecodeError as error:
            raise EditorError(f"invalid JSON: {error}") from error
        if not isinstance(data, dict):
            raise EditorError("JSON body must be an object")
        return data

    def handle_error(self, error: Exception) -> None:
        if isinstance(error, chunks.DataError):
            self.send_json({"error": str(error)}, HTTPStatus.BAD_REQUEST)
        elif isinstance(error, EditorError):
            self.send_json({"error": str(error)}, error.status)
        else:
            self.send_json({"error": repr(error)}, HTTPStatus.INTERNAL_SERVER_ERROR)

    def do_GET(self) -> None:
        try:
            parsed = urlparse(self.path)
            if parsed.path == "/api/meta":
                with STATE.lock:
                    self.send_json(STATE.api_meta())
            elif parsed.path == "/api/rows":
                with STATE.lock:
                    self.send_json(STATE.api_rows(parse_qs(parsed.query)))
            elif parsed.path == "/api/problems":
                with STATE.lock:
                    self.send_json(STATE.api_problems())
            elif parsed.path == "/api/diff":
                with STATE.lock:
                    self.send_json({"diff": STATE.git_diff()})
            elif parsed.path.startswith("/api/"):
                self.send_json({"error": "not found"}, HTTPStatus.NOT_FOUND)
            else:
                self.serve_static(parsed.path)
        except Exception as error:  # noqa: BLE001
            self.handle_error(error)

    def do_POST(self) -> None:
        try:
            parsed = urlparse(self.path)
            payload = self.read_json()
            with STATE.lock:
                routes = {
                    "/api/edit-cell": STATE.api_edit_cell,
                    "/api/add-row": STATE.api_add_row,
                    "/api/soft-remove": STATE.api_soft_remove,
                    "/api/move-rows": STATE.api_move_rows,
                    "/api/bulk-edit": STATE.api_bulk_edit,
                    "/api/revert-row": STATE.api_revert_row,
                    "/api/reload": STATE.api_reload,
                }
                no_payload_routes = {
                    "/api/undo": STATE.api_undo,
                    "/api/redo": STATE.api_redo,
                    "/api/discard-draft": STATE.api_discard_draft,
                    "/api/save-build-check": STATE.api_save_build_check,
                }
                if parsed.path in routes:
                    self.send_json(routes[parsed.path](payload))
                elif parsed.path in no_payload_routes:
                    self.send_json(no_payload_routes[parsed.path]())
                else:
                    self.send_json({"error": "not found"}, HTTPStatus.NOT_FOUND)
        except Exception as error:  # noqa: BLE001
            self.handle_error(error)

    def serve_static(self, request_path: str) -> None:
        relative = "index.html" if request_path in {"", "/"} else request_path.lstrip("/")
        path = (STATIC_DIR / relative).resolve()
        if not str(path).startswith(str(STATIC_DIR.resolve())) or not path.is_file():
            self.send_error(HTTPStatus.NOT_FOUND)
            return
        body = path.read_bytes()
        content_type = mimetypes.guess_type(path.name)[0] or "application/octet-stream"
        if path.suffix == ".js":
            content_type = "application/javascript"
        elif path.suffix == ".css":
            content_type = "text/css"
        self.send_response(HTTPStatus.OK)
        self.send_header("Content-Type", content_type)
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def self_check() -> int:
    required = [
        STATIC_DIR / "index.html",
        STATIC_DIR / "app.js",
        STATIC_DIR / "style.css",
        STATIC_DIR / "vendor" / "tabulator" / "tabulator.min.js",
        STATIC_DIR / "vendor" / "tabulator" / "tabulator.min.css",
    ]
    missing = [path for path in required if not path.exists()]
    if missing:
        for path in missing:
            print(f"missing {path}", file=sys.stderr)
        return 2
    try:
        STATE.scan_chunks()
        print(f"found {len(STATE.chunk_paths)} chunk files")
        print("lexicon editor self-check passed")
        return 0
    except Exception as error:  # noqa: BLE001
        print(error, file=sys.stderr)
        return 2


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--open", action="store_true", help="open the editor in a browser")
    parser.add_argument("--check", action="store_true", help="verify local tool files and exit")
    args = parser.parse_args()
    if args.check:
        return self_check()
    server = ThreadingHTTPServer((args.host, args.port), Handler)
    url = f"http://{args.host}:{server.server_port}/"
    print(f"Lexicon editor running at {url}")
    print("Press Ctrl+C to stop.")
    if args.open:
        webbrowser.open(url)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("")
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
