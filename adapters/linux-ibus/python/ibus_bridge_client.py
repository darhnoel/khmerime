"""JSON-line bridge client for the KhmerIME IBus adapter."""

from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Optional


@dataclass
class BridgeResponse:
    ok: bool
    consumed: bool
    commit_text: Optional[str]
    snapshot: Dict[str, Any]
    error: Optional[str]


class BridgeClient:
    def __init__(self, bridge_path: Path, *, initial_input_mode: str = "roman"):
        self._proc = subprocess.Popen(
            [str(bridge_path), "--initial-input-mode", initial_input_mode],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

    def call(self, payload: Dict[str, Any]) -> BridgeResponse:
        if self._proc.stdin is None or self._proc.stdout is None:
            raise RuntimeError("bridge pipe is unavailable")
        self._proc.stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
        self._proc.stdin.flush()
        line = self._proc.stdout.readline()
        if not line:
            stderr = ""
            if self._proc.stderr is not None:
                stderr = self._proc.stderr.read(4096).strip()
            raise RuntimeError(f"bridge terminated unexpectedly: {stderr}")
        data = json.loads(line)
        return BridgeResponse(
            ok=bool(data.get("ok", False)),
            consumed=bool(data.get("consumed", False)),
            commit_text=data.get("commit_text"),
            snapshot=data.get("snapshot", {}),
            error=data.get("error"),
        )

    def shutdown(self) -> None:
        try:
            self.call({"cmd": "shutdown"})
        except Exception:
            pass
        if self._proc.poll() is None:
            self._proc.terminate()
            try:
                self._proc.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                self._proc.kill()
