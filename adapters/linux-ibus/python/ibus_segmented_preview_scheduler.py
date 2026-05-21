"""Debounced segmented-preview refresh for the KhmerIME IBus adapter.

The segmented preview is built from a WFST shadow observation, which is the
single most expensive operation on the per-keystroke path. Build it after the
user stops typing for a short window rather than on every key.
"""

from __future__ import annotations

from typing import Any, Callable, Dict

from ibus_debounced_bridge_work import DebouncedBridgeWork

SEGMENTED_PREVIEW_SLOW_LOG_MS = 30.0

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
        self._work = DebouncedBridgeWork(
            name="refresh_segmented_preview",
            call_bridge=call_bridge,
            apply_response=apply_response,
            current_raw_preedit=current_raw_preedit,
            log=log,
            timeout_add=timeout_add,
            source_remove=source_remove,
            idle_add=idle_add,
            build_payload=lambda raw_preedit: {
                "cmd": "refresh_segmented_preview",
                "raw_preedit": raw_preedit,
            },
            min_raw_preedit_len=min_raw_preedit_len,
            debounce_ms=debounce_ms,
            slow_log_ms=SEGMENTED_PREVIEW_SLOW_LOG_MS,
            should_apply=lambda response: bool(getattr(response, "ok", False)),
        )

    def cancel(self) -> None:
        self._work.cancel()

    def schedule(self, raw_preedit: str) -> None:
        self._work.schedule(raw_preedit)
