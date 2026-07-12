#!/usr/bin/env python3
"""Local spreadsheet editor for KhmerIME lexicon chunks."""

from __future__ import annotations

import argparse
import json
import mimetypes
import sys
import webbrowser
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

sys.path.insert(0, str(Path(__file__).resolve().parent))

from editor.state import EditorError, EditorState, Row, chunks  # noqa: E402,F401

STATIC_DIR = Path(__file__).resolve().parent / "static"


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
            body = {"error": str(error)}
            if error.detail is not None:
                body["detail"] = error.detail
            self.send_json(body, error.status)
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
                    "/api/duplicate-row": STATE.api_duplicate_row,
                    "/api/soft-remove": STATE.api_soft_remove,
                    "/api/delete-rows": STATE.api_delete_rows,
                    "/api/bulk-regex-preview": STATE.api_bulk_regex_preview,
                    "/api/bulk-regex-apply": STATE.api_bulk_regex_apply,
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
