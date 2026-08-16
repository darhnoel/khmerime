"""Candidate row rendering helpers for the KhmerIME IBus lookup table."""

from __future__ import annotations

from typing import Any

RECOMMENDED_MARK = "✓"
DERIVED_MARK = "≈"

# The Candidate Surface levels. A Segmented Session shows one level at a time:
# whole Phrase Candidates by default, the focused segment's words after Tab.
# A flat Composition has no second level.
FLAT = "flat"
PHRASE = "phrase"
SEGMENT = "segment"


def surface_mode(snapshot: Any) -> str:
    """Which Candidate Surface level the snapshot is asking for."""
    if not isinstance(snapshot, dict):
        return FLAT
    if bool(snapshot.get("segment_edit_active", False)):
        return SEGMENT
    if bool(snapshot.get("segmented_active", False)):
        return PHRASE
    return FLAT


def phrase_rows(snapshot: Any) -> tuple[list[str], list[int], Any]:
    """Phrase-level rows: the whole-composition hypotheses, Khmer only.

    Returns `(rows, session_indices, selected_row)`. Rows are a *filtered*
    subset of `phrase_candidates`, so `session_indices[row]` maps a visible row
    back to the index `select_phrase` expects — the two must never be confused.

    Single-segment entries are dropped: they are first-word guesses, not
    alternative readings of the whole composition. The exception is a one-word
    model rescue, which does span the whole composition. Mirrors macOS's
    `segments.len() >= 2 || from_model` filter.
    """
    if not isinstance(snapshot, dict):
        return [], [], None

    entries = snapshot.get("phrase_candidates")
    if not isinstance(entries, list):
        return [], [], None

    rows: list[str] = []
    indices: list[int] = []
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            continue
        text = str(entry.get("text", "")).strip()
        if not text:
            continue
        segments = entry.get("segments")
        segment_count = len(segments) if isinstance(segments, list) else 0
        if segment_count < 2 and not bool(entry.get("from_model", False)):
            continue
        rows.append(text)
        indices.append(index)

    selected_session_index = snapshot.get("selected_phrase_index", 0)
    selected_row: Any = None
    if isinstance(selected_session_index, int) and selected_session_index in indices:
        selected_row = indices.index(selected_session_index)
    return rows, indices, selected_row


def candidate_rows(candidates: Any, candidate_display: Any, mode: str = FLAT) -> list[str]:
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
        is_raw_fallback = bool(entry.get("is_raw_fallback", False))
        if output.isascii() and not is_raw_fallback:
            continue
        if is_raw_fallback:
            # The raw roman escape hatch (the Commit Rules floor). Show it
            # plainly, without a recommended/derived marker, so the user can
            # always fall back to committing their literal input.
            rendered.append(output)
            continue
        recommended = bool(entry.get("recommended", False))
        hints = [str(hint).strip() for hint in (entry.get("roman_hints") or []) if str(hint).strip()]
        label = output
        if recommended:
            label = f"{RECOMMENDED_MARK} {label}"
        elif not hints:
            label = f"{DERIVED_MARK} {label}"
        # Phrase and Segment rows stay Khmer-only: the segment preview already
        # carries the roman, so repeating it per row costs lookup-table width
        # and adds nothing. Flat mode has no such header, so the row keeps it.
        if hints and mode == FLAT:
            label = f"{label} ({' / '.join(hints[:3])})"
        rendered.append(label)
    return rendered
