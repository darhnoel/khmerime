"""Engine-level behavior: Space-cycling the Candidate List must not re-send
an unchanged Preedit or reload the whole lookup table — only the cursor moves
(via update_lookup_table_fast). Reuses the gi-stub harness from
test_ibus_mode_property; the bridge is faked at _call_bridge_raw, the blessed
monkey-patch seam.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from test_ibus_mode_property import _StubEngineBase, _make_engine


@dataclass
class _Response:
    ok: bool = True
    consumed: bool = True
    commit_text: Optional[str] = None
    snapshot: Dict[str, Any] = field(default_factory=dict)
    readiness: str = "full"
    error: Optional[str] = None
    timings: Optional[Dict[str, float]] = None


def _record_fast_update(self: Any, table: Any, visible: bool) -> None:
    self.__dict__.setdefault("lookup_fast_updates", []).append((table.cursor, len(table.candidates), visible))


_StubEngineBase.update_lookup_table_fast = _record_fast_update


def _snapshot(raw_preedit: str, candidates: List[str], selected: int) -> Dict[str, Any]:
    return {
        "input_mode": "roman",
        "raw_preedit": raw_preedit,
        "preedit": raw_preedit,
        "candidates": candidates,
        "candidate_display": [
            {"output": item, "recommended": index == 0, "roman_hints": [raw_preedit] if index == 0 else []}
            for index, item in enumerate(candidates)
        ],
        "selected_index": selected,
        "segmented_active": False,
        "segment_edit_active": False,
        "segment_preview": [],
    }


def _response(snapshot: Dict[str, Any], commit_text: Optional[str] = None) -> _Response:
    return _Response(commit_text=commit_text, snapshot=snapshot)


def _engine_with_scripted_bridge(responses: List[Any]):
    engine = _make_engine()
    queue = list(responses)
    engine._call_bridge_raw = lambda payload: queue.pop(0)  # noqa: SLF001
    return engine


CANDIDATES = ["សា", "សារ", "ស្អា"]


def test_space_cycling_moves_cursor_without_preedit_or_table_reload():
    engine = _engine_with_scripted_bridge(
        [
            _response(_snapshot("s", ["ស"], 0)),
            _response(_snapshot("sa", CANDIDATES, 0)),
            _response(_snapshot("sa", CANDIDATES, 1)),  # Space: only the cursor moved
            _response(_snapshot("sa", CANDIDATES, 2)),  # Space again
        ]
    )

    engine.do_process_key_event(ord("s"), 0, 0)
    engine.do_process_key_event(ord("a"), 0, 0)
    preedit_updates_after_typing = len(engine.preedit_updates)
    lookup_reloads_after_typing = len(engine.lookup_updates)
    clears_after_typing = engine._table.cleared  # noqa: SLF001

    engine.do_process_key_event(0x20, 0, 0)
    engine.do_process_key_event(0x20, 0, 0)

    assert len(engine.preedit_updates) == preedit_updates_after_typing, "unchanged Preedit must not be re-sent"
    assert len(engine.lookup_updates) == lookup_reloads_after_typing, "unchanged rows must not reload the table"
    assert engine._table.cleared == clears_after_typing, "cycling must not rebuild the table"  # noqa: SLF001
    fast_updates = getattr(engine, "lookup_fast_updates", [])
    assert [cursor for cursor, _, _ in fast_updates] == [1, 2], "each Space is a cursor-only fast update"


def test_identical_composition_after_commit_renders_fully_again():
    engine = _engine_with_scripted_bridge(
        [
            _response(_snapshot("sa", CANDIDATES, 0)),
            _response(_snapshot("", [], None), commit_text="សា"),  # Enter commits
            _response(_snapshot("sa", CANDIDATES, 0)),  # user types the same word again
        ]
    )

    engine.do_process_key_event(ord("a"), 0, 0)
    engine.do_process_key_event(0xFF0D, 0, 0)  # Enter
    preedit_updates_after_commit = len(engine.preedit_updates)
    reloads_after_commit = len(engine.lookup_updates)

    engine.do_process_key_event(ord("a"), 0, 0)

    assert engine.committed_text == ["សា"]
    assert len(engine.preedit_updates) == preedit_updates_after_commit + 1, "post-commit Preedit must render again"
    assert len(engine.lookup_updates) == reloads_after_commit + 1, "post-commit Candidate List must reload"
