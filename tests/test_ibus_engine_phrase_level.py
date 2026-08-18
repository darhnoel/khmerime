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


def test_whole_word_model_surface_keeps_non_model_alternatives():
    from ibus_candidate_renderer import MODEL_MARK

    phrase_candidates = [
        _phrase("ដំរី", 1),
        _phrase("តម្រា", 1, from_model=True),
        _phrase("ដំរ៉ា", 1),
    ]
    for phrase in phrase_candidates:
        phrase["segments"][0]["input"] = "domra"
    before_selection = _phrase_snapshot()
    before_selection["raw_preedit"] = "domra"
    before_selection["preedit"] = "domra"
    before_selection["segmented_active"] = False
    before_selection["phrase_candidates"] = phrase_candidates
    after_selection = _phrase_snapshot(selected_phrase_index=1)
    after_selection["raw_preedit"] = "domra"
    after_selection["preedit"] = "domra"
    after_selection["segmented_active"] = True
    after_selection["segment_preview"] = [
        {"output": "តម្រា", "input": "domra", "focused": True},
    ]
    after_selection["phrase_candidates"] = phrase_candidates
    engine = _recording_engine([_response(before_selection), _response(after_selection)])

    engine.do_process_key_event(ord("v"), 0, 0)
    expected_rows = ["ដំរី (domra)", f"{MODEL_MARK} តម្រា (domra)", "ដំរ៉ា (domra)"]
    assert _painted_rows(engine) == expected_rows

    engine.do_process_key_event(KEY_SPACE, 0, 0)

    assert _painted_rows(engine) == expected_rows
    assert _sent_commands(engine)[-1] == {"cmd": "select_phrase", "index": 1}


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


def test_unverified_model_row_is_painted_with_a_red_marker():
    # ADR-0016: the warning mark must survive the snapshot → adapter path, and red must be
    # distinguishable from white, or unverified output passes as reviewed data.
    from ibus_candidate_renderer import UNVERIFIED_FG, UNVERIFIED_MODEL_MARK

    snapshot = _phrase_snapshot()
    snapshot["phrase_candidates"] = [
        _phrase("ខ្ញុំទៅ", 2),
        _phrase("សំបុក", 1, from_model=True),
    ]
    snapshot["phrase_candidates"][1]["lexicon_verified"] = False
    engine = _recording_engine([_response(snapshot)])

    engine.do_process_key_event(ord("v"), 0, 0)

    rows = _painted_rows(engine)
    assert rows == ["ខ្ញុំទៅ", f"{UNVERIFIED_MODEL_MARK} សំបុក"]

    # The plain Lexicon row carries no attributes; the unverified row colours
    # only its leading marker glyph. The glyph itself is red as a GNOME fallback.
    plain, marked = engine._table.candidates  # noqa: SLF001
    assert not plain.attributes
    assert marked.attributes == [("foreground", UNVERIFIED_FG, 0, len(UNVERIFIED_MODEL_MARK))]


def test_verified_model_row_is_marked_but_not_coloured():
    from ibus_candidate_renderer import MODEL_MARK

    snapshot = _phrase_snapshot()
    snapshot["phrase_candidates"] = [
        _phrase("ខ្ញុំទៅ", 2),
        _phrase("សុខភាព", 1, from_model=True),
    ]
    engine = _recording_engine([_response(snapshot)])

    engine.do_process_key_event(ord("v"), 0, 0)

    assert _painted_rows(engine) == ["ខ្ញុំទៅ", f"{MODEL_MARK} សុខភាព"]
    assert not engine._table.candidates[1].attributes  # noqa: SLF001


def test_space_in_a_flat_composition_still_cycles_candidates():
    snapshot = _phrase_snapshot()
    snapshot["segmented_active"] = False
    snapshot["phrase_candidates"] = []
    engine = _recording_engine([_response(snapshot), _response(snapshot)])

    engine.do_process_key_event(ord("v"), 0, 0)
    engine.do_process_key_event(KEY_SPACE, 0, 0)

    assert _sent_commands(engine)[-1]["cmd"] == "process_key_event"
