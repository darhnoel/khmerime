"""Candidate row rendering helpers for the KhmerIME IBus lookup table."""

from __future__ import annotations

from typing import Any

RECOMMENDED_MARK = "✓"
DERIVED_MARK = "≈"


def candidate_rows(candidates: Any, candidate_display: Any) -> list[str]:
    if not isinstance(candidates, list):
        return []

    rendered = []
    use_display = isinstance(candidate_display, list) and len(candidate_display) == len(candidates)
    for index, candidate in enumerate(candidates):
        text = str(candidate)
        if not use_display:
            if not text.isascii():
                rendered.append(text)
            continue

        entry = candidate_display[index]
        if not isinstance(entry, dict):
            if not text.isascii():
                rendered.append(text)
            continue

        output = str(entry.get("output", "")).strip() or text
        if output.isascii():
            continue
        recommended = bool(entry.get("recommended", False))
        hints = [str(hint).strip() for hint in (entry.get("roman_hints") or []) if str(hint).strip()]
        label = output
        if recommended:
            label = f"{RECOMMENDED_MARK} {label}"
        elif not hints:
            label = f"{DERIVED_MARK} {label}"
        if hints:
            label = f"{label} ({' / '.join(hints[:3])})"
        rendered.append(label)
    return rendered
