"""Shared debounced background bridge work for the KhmerIME IBus adapter."""

from __future__ import annotations

import threading
import time
from typing import Any, Callable, Dict, Optional, Tuple

PayloadBuilder = Callable[[str], Dict[str, Any]]
ResponsePredicate = Callable[[Any], bool]
ResponseHook = Callable[[str, Any], None]
RetryPredicate = Callable[[str, Any], bool]


class DebouncedBridgeWork:
    """Debounce stale bridge work and run only the latest useful request."""

    def __init__(
        self,
        *,
        name: str,
        call_bridge: Callable[[Dict[str, Any]], Any],
        apply_response: Callable[[Any], None],
        current_raw_preedit: Callable[[], str],
        log: Callable[[str], None],
        timeout_add: Callable[..., int],
        source_remove: Callable[[int], None],
        idle_add: Callable[..., Any],
        build_payload: PayloadBuilder,
        min_raw_preedit_len: int,
        debounce_ms: int,
        slow_log_ms: float,
        should_apply: Optional[ResponsePredicate] = None,
        after_apply: Optional[ResponseHook] = None,
        should_retry: Optional[RetryPredicate] = None,
    ):
        self._name = name
        self._call_bridge = call_bridge
        self._apply_response = apply_response
        self._current_raw_preedit = current_raw_preedit
        self._log = log
        self._timeout_add = timeout_add
        self._source_remove = source_remove
        self._idle_add = idle_add
        self._build_payload = build_payload
        self._min_raw_preedit_len = min_raw_preedit_len
        self._debounce_ms = debounce_ms
        self._slow_log_ms = slow_log_ms
        self._should_apply = should_apply or (lambda _response: True)
        self._after_apply = after_apply
        self._should_retry = should_retry
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
            response = self._call_bridge(self._build_payload(raw_preedit))
        except Exception as err:
            self._log(f"{self._name} failed raw_len={len(raw_preedit)} err={err}")
            self._idle_add(self._finish_after_error, generation, raw_preedit)
            return
        elapsed_ms = (time.perf_counter() - started) * 1000.0
        if elapsed_ms >= self._slow_log_ms:
            self._log_slow_call(raw_preedit, elapsed_ms, response)
        self._idle_add(self._finish, generation, raw_preedit, response)

    def _log_slow_call(self, raw_preedit: str, elapsed_ms: float, response: Any) -> None:
        timings = getattr(response, "timings", None) or {}
        br_total = float(timings.get("total_ms", 0.0))
        br_proc = float(timings.get("process_event_ms", 0.0))
        ipc_ms = max(0.0, elapsed_ms - br_total)
        self._log(
            "%s slow raw_len=%s elapsed_ms=%.2f br_total_ms=%.2f proc_ms=%.2f ipc_ms=%.2f"
            % (self._name, len(raw_preedit), elapsed_ms, br_total, br_proc, ipc_ms)
        )

    def _finish_after_error(self, generation: int, raw_preedit: str) -> bool:
        self._finish_run()
        return False

    def _finish(self, generation: int, raw_preedit: str, response: Any) -> bool:
        current_raw = self._current_raw_preedit()
        if generation != self._generation or raw_preedit != current_raw:
            self._log(
                "%s stale raw_len=%s current_len=%s"
                % (self._name, len(raw_preedit), len(current_raw))
            )
            self._finish_run()
            return False
        if self._should_apply(response):
            self._apply_response(response)
            if self._after_apply is not None:
                self._after_apply(raw_preedit, response)
        if self._should_retry is not None and self._should_retry(raw_preedit, response):
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
