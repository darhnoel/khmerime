"""Engine-level behavior at the Phrase level of the Candidate Surface: the rows
IBus paints are whole-phrase hypotheses, and Space/arrows/digits select among
them via the `select_phrase` bridge command rather than cycling word
candidates. Mirrors macOS `CandidateSurface::command_for_key`.
"""

from __future__ import annotations

from typing import Any, Dict, List

from test_ibus_engine_render_dedup import _engine_with_scripted_bridge, _response

KEY_SPACE = 0x20
KEY_UP = 0xFF52
KEY_DOWN = 0xFF54


def _phrase(text: str, segments: int, from_model: bool = False) -> Dict[str, Any]:
    return {
        "text": text,
        "segments": [{"output": text, "input": "x"} for _ in range(segments)],
        "from_model": from_model,
        "lexicon_verified": True,
    }


def _phrase_snapshot(selected_phrase_index: int = 0) -> Dict[str, Any]:
    return {
        "input_mode": "roman",
        "raw_preedit": "nhomtov",
        "preedit": "nhomtov",
        # Word-level candidates for the focused segment. At the Phrase level
        # these must NOT be what the lookup table shows.
        "candidates": ["ខ្ញុំ", "ខ្ចុំ"],
        "candidate_display": [
            {"output": "ខ្ញុំ", "recommended": True, "roman_hints": ["nhom"]},
            {"output": "ខ្ចុំ", "recommended": False, "roman_hints": ["nhom"]},
        ],
        "selected_index": 0,
        "segmented_active": True,
        "segment_edit_active": False,
        "segment_preview": [
            {"output": "ខ្ញុំ", "input": "nhom", "focused": True},
            {"output": "ទៅ", "input": "tov", "focused": False},
        ],
        "focused_segment_index": 0,
        "phrase_candidates": [_phrase("ខ្ញុំទៅ", 2), _phrase("ខ្ញុំតៅ", 2)],
        "selected_phrase_index": selected_phrase_index,
    }


def _painted_rows(engine: Any) -> List[str]:
    return [candidate.get_text() for candidate in engine._table.candidates]  # noqa: SLF001


def _sent_commands(engine: Any) -> List[Dict[str, Any]]:
    return engine.__dict__.setdefault("sent_payloads", [])


def _recording_engine(responses: List[Any]):
    """Wrap the scripted bridge so we can assert on the commands Python sends."""
    engine = _engine_with_scripted_bridge(responses)
    inner = engine._call_bridge_raw  # noqa: SLF001

    def record(payload: Dict[str, Any]):
        engine.__dict__.setdefault("sent_payloads", []).append(payload)
        return inner(payload)

    engine._call_bridge_raw = record  # noqa: SLF001
    return engine


def test_phrase_level_paints_whole_phrase_rows_not_word_candidates():
    engine = _recording_engine([_response(_phrase_snapshot())])

    engine.do_process_key_event(ord("v"), 0, 0)

    assert _painted_rows(engine) == ["ខ្ញុំទៅ", "ខ្ញុំតៅ"]


def test_space_at_the_phrase_level_selects_the_next_phrase():
    engine = _recording_engine(
        [
            _response(_phrase_snapshot(selected_phrase_index=0)),
            _response(_phrase_snapshot(selected_phrase_index=1)),
        ]
    )

    engine.do_process_key_event(ord("v"), 0, 0)
    consumed = engine.do_process_key_event(KEY_SPACE, 0, 0)

    assert consumed is True
    assert _sent_commands(engine)[-1] == {"cmd": "select_phrase", "index": 1}


def test_down_arrow_selects_the_next_phrase_and_up_wraps_backwards():
    engine = _recording_engine(
        [
            _response(_phrase_snapshot(selected_phrase_index=0)),
            _response(_phrase_snapshot(selected_phrase_index=1)),
            _response(_phrase_snapshot(selected_phrase_index=0)),
        ]
    )

    engine.do_process_key_event(ord("v"), 0, 0)
    engine.do_process_key_event(KEY_DOWN, 0, 0)
    assert _sent_commands(engine)[-1] == {"cmd": "select_phrase", "index": 1}

    engine.do_process_key_event(KEY_UP, 0, 0)
    assert _sent_commands(engine)[-1] == {"cmd": "select_phrase", "index": 0}


def test_digit_selects_the_phrase_on_that_row():
    engine = _recording_engine(
        [
            _response(_phrase_snapshot(selected_phrase_index=0)),
            _response(_phrase_snapshot(selected_phrase_index=1)),
        ]
    )

    engine.do_process_key_event(ord("v"), 0, 0)
    engine.do_process_key_event(ord("2"), 0, 0)

    assert _sent_commands(engine)[-1] == {"cmd": "select_phrase", "index": 1}


def test_digit_past_the_last_row_falls_through_to_the_bridge():
    engine = _recording_engine(
        [
            _response(_phrase_snapshot(selected_phrase_index=0)),
            _response(_phrase_snapshot(selected_phrase_index=0)),
        ]
    )

    engine.do_process_key_event(ord("v"), 0, 0)
    engine.do_process_key_event(ord("9"), 0, 0)

    # No 9th phrase: the key stays an ordinary key event, not a bad selection.
    assert _sent_commands(engine)[-1]["cmd"] == "process_key_event"


def test_enter_still_commits_at_the_phrase_level():
    engine = _recording_engine(
        [
            _response(_phrase_snapshot(selected_phrase_index=0)),
            _response(_phrase_snapshot(), commit_text="ខ្ញុំទៅ"),
        ]
    )

    engine.do_process_key_event(ord("v"), 0, 0)
    engine.do_process_key_event(0xFF0D, 0, 0)

    assert _sent_commands(engine)[-1]["cmd"] == "process_key_event"
    assert engine.committed_text == ["ខ្ញុំទៅ"]


def test_space_in_segment_edit_mode_still_cycles_words():
    # Segment level is unchanged: Space cycles the focused word via the shared
    # session (ADR-0003), so it must stay an ordinary key event.
    snapshot = _phrase_snapshot()
    snapshot["segment_edit_active"] = True
    engine = _recording_engine([_response(snapshot), _response(snapshot)])

    engine.do_process_key_event(ord("v"), 0, 0)
    engine.do_process_key_event(KEY_SPACE, 0, 0)

    assert _sent_commands(engine)[-1]["cmd"] == "process_key_event"


def test_space_in_a_flat_composition_still_cycles_candidates():
    snapshot = _phrase_snapshot()
    snapshot["segmented_active"] = False
    snapshot["phrase_candidates"] = []
    engine = _recording_engine([_response(snapshot), _response(snapshot)])

    engine.do_process_key_event(ord("v"), 0, 0)
    engine.do_process_key_event(KEY_SPACE, 0, 0)

    assert _sent_commands(engine)[-1]["cmd"] == "process_key_event"
