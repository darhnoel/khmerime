"""Dedup planner for IBus Preedit and Candidate List updates.

While the user cycles the Candidate List, the session snapshot changes only
in `selected_index`; re-sending an identical Preedit and reloading the whole
lookup table on every keypress is what makes cycling feel laggy. The engine
asks this planner what actually changed and sends only that.
"""

from __future__ import annotations

from typing import Any, Optional, Sequence, Tuple

RELOAD = "reload"
CURSOR = "cursor"
SKIP = "skip"


class RenderPlanner:
    def __init__(self) -> None:
        self._last_preedit_key: Optional[Tuple[Any, ...]] = None
        self._last_rows: Optional[Tuple[str, ...]] = None
        self._last_cursor: Optional[int] = None

    def reset(self) -> None:
        """Forget sent state after commit/hide so the next snapshot renders fully."""
        self._last_preedit_key = None
        self._last_rows = None
        self._last_cursor = None

    def preedit_changed(self, preedit_key: Tuple[Any, ...]) -> bool:
        if preedit_key == self._last_preedit_key:
            return False
        self._last_preedit_key = preedit_key
        return True

    def candidate_list_action(self, rows: Sequence[str], cursor: Optional[int]) -> str:
        rows_key = tuple(rows)
        if rows_key == self._last_rows:
            if cursor == self._last_cursor:
                return SKIP
            self._last_cursor = cursor
            return CURSOR
        self._last_rows = rows_key
        self._last_cursor = cursor
        return RELOAD
