"""Pure helpers for IBus segment preview text and chunk spans."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Iterable, Optional

SEGMENT_SEPARATOR = " | "
FOCUSED_MARKER_OPEN = "⟦"
FOCUSED_MARKER_CLOSE = "⟧"
FOCUSED_MARKER_MODE = "double_bracket"


@dataclass(frozen=True)
class SegmentSpan:
    """Text range for one rendered segment chunk."""

    start: int
    end: int
    focused: bool


def build_segment_preview(
    entries: Iterable[Any], separator: str = SEGMENT_SEPARATOR
) -> tuple[str, list[SegmentSpan], Optional[int]]:
    """Build preview text + spans from bridge segment preview entries."""
    parts: list[str] = []
    spans: list[SegmentSpan] = []
    focused_index: Optional[int] = None
    cursor = 0

    for entry in entries:
        if not isinstance(entry, dict):
            continue
        output = str(entry.get("output", "")).strip()
        if not output:
            continue
        input_roman = str(entry.get("input", "")).strip()
        segment = output if not input_roman else f"{output}({input_roman})"
        focused = bool(entry.get("focused", False))
        if focused:
            segment = f"{FOCUSED_MARKER_OPEN}{segment}{FOCUSED_MARKER_CLOSE}"

        if parts:
            cursor += len(separator)
        start = cursor
        end = start + len(segment)
        parts.append(segment)
        spans.append(SegmentSpan(start=start, end=end, focused=focused))
        if focused and focused_index is None:
            focused_index = len(spans) - 1
        cursor = end

    return separator.join(parts), spans, focused_index


def build_segment_preview_fallback(entries: Iterable[Any]) -> str:
    """Plain-text preview used when styled rendering is unavailable."""
    text, _, _ = build_segment_preview(entries)
    return text
