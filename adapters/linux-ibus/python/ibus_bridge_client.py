"""JSON-line bridge client for the KhmerIME IBus adapter."""

from __future__ import annotations

import collections
import json
import subprocess
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Deque, Dict, Optional


@dataclass
class BridgeResponse:
    ok: bool
    consumed: bool
    commit_text: Optional[str]
    readiness: str
    snapshot: Dict[str, Any]
    error: Optional[str]
    timings: Optional[Dict[str, float]] = None
    refinement_pending: bool = False


class BridgeClient:
    def __init__(
        self,
        bridge_path: Path,
        *,
        initial_input_mode: str = "roman",
        deferred_segmented_preview: bool = False,
        log: Optional[Callable[[str], None]] = None,
    ):
        args = [str(bridge_path), "--initial-input-mode", initial_input_mode]
        if deferred_segmented_preview:
            args.append("--deferred-segmented-preview")
        self._proc = subprocess.Popen(
            args,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        self._log = log
        self._recent_stderr: Deque[str] = collections.deque(maxlen=32)
        self._stderr_lock = threading.Lock()
        self._stderr_thread: Optional[threading.Thread] = None
        if self._proc.stderr is not None:
            self._stderr_thread = threading.Thread(
                target=self._drain_stderr,
                name="khmerime-bridge-stderr",
                daemon=True,
            )
            self._stderr_thread.start()

    def _drain_stderr(self) -> None:
        stream = self._proc.stderr
        if stream is None:
            return
        try:
            for raw in stream:
                line = raw.rstrip("\n")
                if not line:
                    continue
                with self._stderr_lock:
                    self._recent_stderr.append(line)
                if self._log is not None:
                    try:
                        self._log(f"bridge_stderr {line}")
                    except Exception:
                        pass
        except (ValueError, OSError):
            pass

    def _snapshot_recent_stderr(self) -> str:
        with self._stderr_lock:
            return "\n".join(self._recent_stderr)

    def call(self, payload: Dict[str, Any]) -> BridgeResponse:
        if self._proc.stdin is None or self._proc.stdout is None:
            raise RuntimeError("bridge pipe is unavailable")
        self._proc.stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
        self._proc.stdin.flush()
        line = self._proc.stdout.readline()
        if not line:
            stderr = self._snapshot_recent_stderr()
            raise RuntimeError(f"bridge terminated unexpectedly: {stderr}")
        data = json.loads(line)
        return BridgeResponse(
            ok=bool(data.get("ok", False)),
            consumed=bool(data.get("consumed", False)),
            commit_text=data.get("commit_text"),
            readiness=str(data.get("readiness", "unknown")),
            snapshot=data.get("snapshot", {}),
            error=data.get("error"),
            timings=data.get("timings"),
            refinement_pending=bool(data.get("refinement_pending", False)),
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
