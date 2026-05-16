"""Debounced long-composition refinement for the KhmerIME IBus adapter."""

from __future__ import annotations

import threading
import time
from typing import Any, Callable, Dict, Optional, Tuple

REFINE_MIN_RAW_PREEDIT_LEN = 10
REFINE_DEBOUNCE_MS = 400
REFINE_SLOW_LOG_MS = 100.0


class RefinementScheduler:
    def __init__(
        self,
        call_refine: Callable[[Dict[str, Any]], Any],
        apply_response: Callable[[Any], None],
        current_raw_preedit: Callable[[], str],
        log: Callable[[str], None],
        timeout_add: Callable[..., int],
        source_remove: Callable[[int], None],
        idle_add: Callable[..., Any],
        min_raw_preedit_len: int = REFINE_MIN_RAW_PREEDIT_LEN,
        debounce_ms: int = REFINE_DEBOUNCE_MS,
        slow_log_ms: float = REFINE_SLOW_LOG_MS,
    ):
        self._call_refine = call_refine
        self._apply_response = apply_response
        self._current_raw_preedit = current_raw_preedit
        self._log = log
        self._timeout_add = timeout_add
        self._source_remove = source_remove
        self._idle_add = idle_add
        self._min_raw_preedit_len = min_raw_preedit_len
        self._debounce_ms = debounce_ms
        self._slow_log_ms = slow_log_ms
        self._timeout_id = 0
        self._generation = 0
        self._lock = threading.Lock()
        self._running = False
        self._pending: Optional[Tuple[int, str]] = None

    def cancel(self) -> None:
        self._generation += 1
        if self._timeout_id:
            self._source_remove(self._timeout_id)
            self._timeout_id = 0
        with self._lock:
            self._pending = None

    def schedule(self, raw_preedit: str) -> None:
        if len(raw_preedit) < self._min_raw_preedit_len:
            return
        if self._timeout_id:
            self._source_remove(self._timeout_id)
            self._timeout_id = 0
        generation = self._generation
        self._timeout_id = self._timeout_add(
            self._debounce_ms,
            self._start,
            generation,
            raw_preedit,
        )

    def _start(self, generation: int, raw_preedit: str) -> bool:
        self._timeout_id = 0
        if not self._begin_run(generation, raw_preedit):
            return False
        self._spawn_run(generation, raw_preedit)
        return False

    def _begin_run(self, generation: int, raw_preedit: str) -> bool:
        with self._lock:
            if self._running:
                self._pending = (generation, raw_preedit)
                return False
            self._running = True
            return True

    def _spawn_run(self, generation: int, raw_preedit: str) -> None:
        threading.Thread(
            target=self._run,
            args=(generation, raw_preedit),
            daemon=True,
        ).start()

    def _run(self, generation: int, raw_preedit: str) -> None:
        started = time.perf_counter()
        try:
            response = self._call_refine(
                {
                    "cmd": "refine_composition",
                    "raw_preedit": raw_preedit,
                }
            )
        except Exception as err:
            self._log(f"refine_composition failed raw_len={len(raw_preedit)} err={err}")
            self._idle_add(self._finish_after_error, generation, raw_preedit)
            return
        elapsed_ms = (time.perf_counter() - started) * 1000.0
        if elapsed_ms >= self._slow_log_ms:
            self._log(
                "refine_composition slow raw_len=%s elapsed_ms=%.2f"
                % (len(raw_preedit), elapsed_ms)
            )
        self._idle_add(self._finish, generation, raw_preedit, response)

    def _finish_after_error(self, generation: int, raw_preedit: str) -> bool:
        self._finish_run()
        return False

    def _finish(self, generation: int, raw_preedit: str, response: Any) -> bool:
        current_raw = self._current_raw_preedit()
        if generation != self._generation or raw_preedit != current_raw:
            self._log(
                "refine_composition stale raw_len=%s current_len=%s"
                % (len(raw_preedit), len(current_raw))
            )
            self._finish_run()
            return False
        self._apply_response(response)
        if response.error:
            self._log(f"bridge error payload=refine_composition error={response.error}")
        else:
            self._log(
                "refine_composition applied raw_len=%s cand=%s"
                % (len(raw_preedit), len(response.snapshot.get("candidates", []) or []))
            )
        readiness = getattr(response, "readiness", "unknown")
        if readiness == "phase_a" and raw_preedit == self._current_raw_preedit():
            self._log(
                "refine_composition retry readiness=%s raw_len=%s"
                % (readiness, len(raw_preedit))
            )
            self.schedule(raw_preedit)
        self._finish_run()
        return False

    def _finish_run(self) -> None:
        next_request: Optional[Tuple[int, str]] = None
        with self._lock:
            self._running = False
            if self._pending is not None:
                generation, raw_preedit = self._pending
                self._pending = None
                if generation == self._generation and raw_preedit == self._current_raw_preedit():
                    self._running = True
                    next_request = (generation, raw_preedit)
        if next_request is not None:
            self._spawn_run(*next_request)
