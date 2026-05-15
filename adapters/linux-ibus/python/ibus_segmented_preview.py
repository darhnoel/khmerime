"""Debounced segmented-preview refresh for the KhmerIME IBus adapter.

The segmented preview is built from a WFST shadow observation, which is the
single most expensive operation on the per-keystroke path. Build it after the
user stops typing for a short window rather than on every key.
"""

from __future__ import annotations

import threading
from typing import Any, Callable, Dict

SEGMENTED_PREVIEW_MIN_RAW_PREEDIT_LEN = 4
SEGMENTED_PREVIEW_DEBOUNCE_MS = 220


class SegmentedPreviewScheduler:
    def __init__(
        self,
        call_bridge: Callable[[Dict[str, Any]], Any],
        apply_response: Callable[[Any], None],
        current_raw_preedit: Callable[[], str],
        log: Callable[[str], None],
        timeout_add: Callable[..., int],
        source_remove: Callable[[int], None],
        idle_add: Callable[..., Any],
        min_raw_preedit_len: int = SEGMENTED_PREVIEW_MIN_RAW_PREEDIT_LEN,
        debounce_ms: int = SEGMENTED_PREVIEW_DEBOUNCE_MS,
    ):
        self._call_bridge = call_bridge
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
            response = self._call_bridge(
                {
                    "cmd": "refresh_segmented_preview",
                    "raw_preedit": raw_preedit,
                }
            )
        except Exception as err:
            self._log(
                f"refresh_segmented_preview failed raw_len={len(raw_preedit)} err={err}"
            )
            return
        self._idle_add(self._finish, generation, raw_preedit, response)

    def _finish(self, generation: int, raw_preedit: str, response: Any) -> bool:
        if generation != self._generation:
            return False
        if raw_preedit != self._current_raw_preedit():
            return False
        if not getattr(response, "ok", False):
            return False
        self._apply_response(response)
        return False
