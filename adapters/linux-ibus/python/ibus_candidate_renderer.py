"""Candidate row rendering helpers for the KhmerIME IBus lookup table."""

from __future__ import annotations

from typing import Any

RECOMMENDED_MARK = "✓"
DERIVED_MARK = "≈"

# Model-provenance marker (ADR-0016 / ADR-0019). Load-bearing UI, not
# decoration: without it, unverified model output would be indistinguishable
# from human-reviewed Lexicon data. White ✦ = model-assisted but Lexicon
# verified; red marker = the model produced Khmer that is not in the Lexicon.
# GNOME Shell discards foreground attributes on lookup-table candidates, unlike
# the custom iOS and Android candidate views, so Linux uses a red Unicode marker
# as the visible fallback. Red rows stay visible and selectable on purpose — the
# Lexicon gate is a marker, not a filter, and an out-of-Lexicon word may be a
# name or loanword.
MODEL_MARK = "✦"
UNVERIFIED_MODEL_MARK = "🔻"
UNVERIFIED_FG = 0xCC0000
MAX_VISIBLE_PHRASE_CANDIDATES = 5

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
    entries = snapshot.get("phrase_candidates")
    if isinstance(entries, list) and any(
        isinstance(entry, dict) and bool(entry.get("from_model", False))
        for entry in entries
    ):
        # A successful whole-word model winner intentionally has no multi-chunk
        # Segmented Session. Keep it on the Phrase surface anyway so its
        # provenance survives as the mandatory ✦ marker (ADR-0016).
        return PHRASE
    return FLAT


def phrase_rows(snapshot: Any) -> tuple[list[str], list[int], Any]:
    """Phrase-level rows: the whole-composition hypotheses, Khmer only.

    Returns `(rows, session_indices, selected_row)`. Rows are a *filtered*
    subset of `phrase_candidates`, so `session_indices[row]` maps a visible row
    back to the index `select_phrase` expects — the two must never be confused.

    In a Segmented Session, ordinary single-segment entries are dropped because
    they are first-word guesses, not alternative readings of the whole
    composition. A model-refined list is different: keep its ordinary
    alternatives alongside the marked model results, even after selecting a
    phrase creates a Segmented Session. Selection must move the cursor without
    collapsing the list to model-only rows.
    """
    if not isinstance(snapshot, dict):
        return [], [], None

    entries = snapshot.get("phrase_candidates")
    if not isinstance(entries, list):
        return [], [], None

    keep_model_list_alternatives = any(
        isinstance(entry, dict) and bool(entry.get("from_model", False))
        for entry in entries
    )

    rows: list[str] = []
    indices: list[int] = []
    raw_preedit = str(snapshot.get("raw_preedit", ""))
    model_rows: list[bool] = []
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            continue
        text = str(entry.get("text", "")).strip()
        if not text:
            continue
        raw_segments = entry.get("segments")
        segments = raw_segments if isinstance(raw_segments, list) else []
        segment_count = len(segments)
        from_model = bool(entry.get("from_model", False))
        if segment_count < 2 and not from_model and not keep_model_list_alternatives:
            continue
        # A model-assisted reading is marked so it can never pass as
        # human-reviewed Lexicon data (ADR-0016). GNOME Shell flattens IBus
        # candidate text and drops its attributes, so an unverified candidate
        # needs a visibly red glyph in the string itself.
        if from_model:
            marker = (
                MODEL_MARK
                if bool(entry.get("lexicon_verified", True))
                else UNVERIFIED_MODEL_MARK
            )
            label = f"{marker} {text}"
        else:
            label = text

        input_parts: list[str] = []
        roman_parts: list[str] = []
        has_lexicon_hints = False
        for segment in segments:
            if not isinstance(segment, dict):
                continue
            input_roman = str(segment.get("input", "")).strip()
            input_parts.append(input_roman)
            hints = [
                str(hint).strip()
                for hint in (segment.get("roman_hints") or [])
                if str(hint).strip()
            ][:3]
            if hints:
                has_lexicon_hints = True
                roman_parts.append(" / ".join(hints))
            else:
                roman_parts.append(input_roman)
        if (
            len(roman_parts) == segment_count
            and all(roman_parts)
            and (
                has_lexicon_hints
                or (
                    raw_preedit
                    and "".join(input_parts) == raw_preedit
                )
            )
        ):
            label = f"{label} ({' · '.join(roman_parts)})"

        rows.append(label)
        indices.append(index)
        model_rows.append(from_model)

    rows, indices = _cap_keeping_a_model_rescue(rows, indices, model_rows)

    selected_session_index = snapshot.get("selected_phrase_index", 0)
    selected_row: Any = None
    if isinstance(selected_session_index, int) and selected_session_index in indices:
        selected_row = indices.index(selected_session_index)
    return rows, indices, selected_row


def _cap_keeping_a_model_rescue(
    rows: list[str], indices: list[int], model_rows: list[bool]
) -> tuple[list[str], list[int]]:
    """Trim to `MAX_VISIBLE_PHRASE_CANDIDATES`, never truncating away a model rescue.

    A **Word Rescuer** proposal can rank below the cap: the word is absent from the
    Lexicon — the reason the model was needed at all — so it carries no frequency
    prior and ordinary readings out-score it. Those ordinary single-segment readings
    are themselves only eligible *because* a model candidate exists, so a plain
    truncation lets them fill every slot and evict the rescue that admitted them,
    making it unreachable rather than merely low.

    The cap still holds: the rescue takes the last slot, it does not add one.
    """
    if len(rows) <= MAX_VISIBLE_PHRASE_CANDIDATES:
        return rows, indices

    kept_rows = rows[:MAX_VISIBLE_PHRASE_CANDIDATES]
    kept_indices = indices[:MAX_VISIBLE_PHRASE_CANDIDATES]
    if any(model_rows[:MAX_VISIBLE_PHRASE_CANDIDATES]):
        return kept_rows, kept_indices  # a rescue already survived the cut

    rescue = next((row for row, is_model in enumerate(model_rows) if is_model), None)
    if rescue is None:
        return kept_rows, kept_indices  # nothing model-assisted to protect

    kept_rows[-1] = rows[rescue]
    kept_indices[-1] = indices[rescue]
    return kept_rows, kept_indices


def phrase_provenance(snapshot: Any, indices: list[int]) -> list[tuple[bool, bool]]:
    """`(from_model, lexicon_verified)` per visible row, in row order.

    Read from the same `phrase_candidates` entries the rows came from, so the
    flags cannot drift from the text. A missing entry defaults to a plain
    Lexicon candidate.
    """
    if not isinstance(snapshot, dict):
        return []
    entries = snapshot.get("phrase_candidates")
    if not isinstance(entries, list):
        return []

    flags: list[tuple[bool, bool]] = []
    for index in indices:
        entry = entries[index] if 0 <= index < len(entries) else None
        if not isinstance(entry, dict):
            flags.append((False, True))
            continue
        flags.append(
            (
                bool(entry.get("from_model", False)),
                bool(entry.get("lexicon_verified", True)),
            )
        )
    return flags


def marker_spans(rows: list[str], flags: list[tuple[bool, bool]]) -> list[tuple[int, int, int, int]]:
    """Foreground colour spans for unverified model markers.

    Returns `(row, colour, start, end)` per row needing colour. The leading
    warning glyph also receives a foreground attribute for IBus panels that
    support it; GNOME Shell ignores that attribute but still renders the red
    Unicode glyph. The Khmer text keeps the default label colour. A verified
    model marker needs no span, so the list is empty in the common case.
    """
    spans: list[tuple[int, int, int, int]] = []
    for row, text in enumerate(rows):
        if row >= len(flags):
            break
        from_model, lexicon_verified = flags[row]
        if not from_model or lexicon_verified:
            continue
        if not text.startswith(UNVERIFIED_MODEL_MARK):
            continue
        spans.append((row, UNVERIFIED_FG, 0, len(UNVERIFIED_MODEL_MARK)))
    return spans


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
