"""Debounced long-composition refinement for the KhmerIME IBus adapter."""

from __future__ import annotations

import threading
from typing import Any, Callable, Dict

REFINE_MIN_RAW_PREEDIT_LEN = 10
REFINE_DEBOUNCE_MS = 400


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
        self._timeout_id = 0
        self._generation = 0

    def cancel(self) -> None:
        self._generation += 1
        if self._timeout_id:
            self._source_remove(self._timeout_id)
            self._timeout_id = 0

    def schedule(self, raw_preedit: str) -> None:
        if len(raw_preedit) < self._min_raw_preedit_len:
            return
        generation = self._generation
        self._timeout_id = self._timeout_add(
            self._debounce_ms,
            self._start,
            generation,
            raw_preedit,
        )

    def _start(self, generation: int, raw_preedit: str) -> bool:
        self._timeout_id = 0
        threading.Thread(
            target=self._run,
            args=(generation, raw_preedit),
            daemon=True,
        ).start()
        return False

    def _run(self, generation: int, raw_preedit: str) -> None:
        try:
            response = self._call_refine(
                {
                    "cmd": "refine_composition",
                    "raw_preedit": raw_preedit,
                }
            )
        except Exception as err:
            self._log(f"refine_composition failed raw_len={len(raw_preedit)} err={err}")
            return
        self._idle_add(self._finish, generation, raw_preedit, response)

    def _finish(self, generation: int, raw_preedit: str, response: Any) -> bool:
        current_raw = self._current_raw_preedit()
        if generation != self._generation or raw_preedit != current_raw:
            self._log(
                "refine_composition stale raw_len=%s current_len=%s"
                % (len(raw_preedit), len(current_raw))
            )
            return False
        self._apply_response(response)
        if response.error:
            self._log(f"bridge error payload=refine_composition error={response.error}")
        else:
            self._log(
                "refine_composition applied raw_len=%s cand=%s"
                % (len(raw_preedit), len(response.snapshot.get("candidates", []) or []))
            )
        return False
