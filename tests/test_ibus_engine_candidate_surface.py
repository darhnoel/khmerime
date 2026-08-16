"""Engine-level behavior: the rows IBus actually paints carry roman only in
Flat mode. In a Segmented Session the roman lives in the segment preview (aux
text), so Phrase and Segment rows are Khmer-only — the two-level Candidate
Surface port (linux-ibus ADR-0001). Reuses the gi-stub harness from
test_ibus_mode_property; the bridge is faked at _call_bridge_raw.
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional

from test_ibus_engine_render_dedup import _engine_with_scripted_bridge, _response


def _segmented_snapshot(
    raw_preedit: str,
    candidates: List[str],
    *,
    segmented_active: bool,
    segment_edit_active: bool,
    hints: Optional[List[str]] = None,
) -> Dict[str, Any]:
    hints = hints if hints is not None else [raw_preedit]
    return {
        "input_mode": "roman",
        "raw_preedit": raw_preedit,
        "preedit": raw_preedit,
        "candidates": candidates,
        "candidate_display": [
            {"output": item, "recommended": index == 0, "roman_hints": hints}
            for index, item in enumerate(candidates)
        ],
        "selected_index": 0,
        "segmented_active": segmented_active,
        "segment_edit_active": segment_edit_active,
        "segment_preview": [{"output": item, "input": raw_preedit, "focused": index == 0} for index, item in enumerate(candidates)]
        if segmented_active
        else [],
        "focused_segment_index": 0 if segmented_active else None,
    }


def _painted_rows(engine: Any) -> List[str]:
    return [candidate.get_text() for candidate in engine._table.candidates]  # noqa: SLF001


def test_flat_composition_paints_roman_on_the_row():
    engine = _engine_with_scripted_bridge(
        [
            _response(
                _segmented_snapshot("sala", ["សាលា"], segmented_active=False, segment_edit_active=False)
            )
        ]
    )

    engine.do_process_key_event(ord("a"), 0, 0)

    assert _painted_rows(engine) == ["✓ សាលា (sala)"]


def test_segmented_session_without_phrase_candidates_paints_no_rows():
    # Phrase level sources rows from `phrase_candidates`, never from the focused
    # segment's word `candidates` — painting those here is what made a word
    # alternative look like a wrong whole-phrase alternative. Phrase-level row
    # content is covered in test_ibus_engine_phrase_level.
    engine = _engine_with_scripted_bridge(
        [
            _response(
                _segmented_snapshot(
                    "somtov", ["សំទៅ", "សុំទៅ"], segmented_active=True, segment_edit_active=False
                )
            )
        ]
    )

    engine.do_process_key_event(ord("v"), 0, 0)

    assert _painted_rows(engine) == []


def test_segment_edit_mode_paints_khmer_only_rows():
    engine = _engine_with_scripted_bridge(
        [
            _response(
                _segmented_snapshot(
                    "somtov", ["សំ", "សុំ"], segmented_active=True, segment_edit_active=True
                )
            )
        ]
    )

    engine.do_process_key_event(ord("v"), 0, 0)

    assert _painted_rows(engine) == ["✓ សំ", "សុំ"]
