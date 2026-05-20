"""Debounced long-composition refinement for the KhmerIME IBus adapter."""

from __future__ import annotations

from typing import Any, Callable, Dict

from ibus_debounced_bridge_work import DebouncedBridgeWork

REFINE_MIN_RAW_PREEDIT_LEN = 10
REFINE_DEBOUNCE_MS = 400
REFINE_SLOW_LOG_MS = 30.0


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
        self._current_raw_preedit = current_raw_preedit
        self._log = log
        self._work = DebouncedBridgeWork(
            name="refine_composition",
            call_bridge=call_refine,
            apply_response=apply_response,
            current_raw_preedit=current_raw_preedit,
            log=log,
            timeout_add=timeout_add,
            source_remove=source_remove,
            idle_add=idle_add,
            build_payload=lambda raw_preedit: {
                "cmd": "refine_composition",
                "raw_preedit": raw_preedit,
            },
            min_raw_preedit_len=min_raw_preedit_len,
            debounce_ms=debounce_ms,
            slow_log_ms=slow_log_ms,
            after_apply=self._after_apply,
            should_retry=self._should_retry,
        )

    def cancel(self) -> None:
        self._work.cancel()

    def schedule(self, raw_preedit: str) -> None:
        self._work.schedule(raw_preedit)

    def _after_apply(self, raw_preedit: str, response: Any) -> None:
        if response.error:
            self._log(f"bridge error payload=refine_composition error={response.error}")
        else:
            self._log(
                "refine_composition applied raw_len=%s cand=%s"
                % (len(raw_preedit), len(response.snapshot.get("candidates", []) or []))
            )

    def _should_retry(self, raw_preedit: str, response: Any) -> bool:
        readiness = getattr(response, "readiness", "unknown")
        if readiness == "phase_a" and raw_preedit == self._current_raw_preedit():
            self._log(
                "refine_composition retry readiness=%s raw_len=%s"
                % (readiness, len(raw_preedit))
            )
            return True
        return False
