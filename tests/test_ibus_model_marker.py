"""The ✦ model-provenance marker (ADR-0016 / ADR-0019).

The marker is load-bearing UI, not decoration: without it, unverified model
output would be indistinguishable from human-reviewed Lexicon data.

  no ✦      plain Lexicon candidate (from_model = false)
  white ✦   from_model && lexicon_verified — model-assisted but trusted
  red ✦     from_model && !lexicon_verified — visibly unverified, still shown

Red candidates stay selectable on purpose: an out-of-Lexicon model word may be
a name or loanword the user genuinely wants. The Lexicon gate is a marker, not
a filter.
"""

from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_candidate_renderer import (  # noqa: E402
    MODEL_MARK,
    UNVERIFIED_FG,
    marker_spans,
    phrase_rows,
)


def _phrase(text: str, segments: int, from_model: bool = False, lexicon_verified: bool = True) -> dict:
    return {
        "text": text,
        "segments": [{"output": text, "input": "x"} for _ in range(segments)],
        "from_model": from_model,
        "lexicon_verified": lexicon_verified,
    }


def test_plain_lexicon_phrase_gets_no_marker():
    snapshot = {"phrase_candidates": [_phrase("ខ្ញុំទៅ", 2)], "selected_phrase_index": 0}

    rows, _, _ = phrase_rows(snapshot)

    assert rows == ["ខ្ញុំទៅ"]


def test_verified_model_phrase_gets_a_marker():
    snapshot = {
        "phrase_candidates": [_phrase("សុខភាព", 1, from_model=True, lexicon_verified=True)],
        "selected_phrase_index": 0,
    }

    rows, _, _ = phrase_rows(snapshot)

    assert rows == [f"{MODEL_MARK} សុខភាព"]


def test_unverified_model_phrase_is_shown_not_filtered():
    # ADR-0016: the Lexicon gate is a marker, not a filter. An out-of-Lexicon
    # model word may be a word the user genuinely wants that the Lexicon does
    # not carry yet.
    snapshot = {
        "phrase_candidates": [_phrase("សំបុក", 1, from_model=True, lexicon_verified=False)],
        "selected_phrase_index": 0,
    }

    rows, indices, _ = phrase_rows(snapshot)

    assert rows == [f"{MODEL_MARK} សំបុក"]
    assert indices == [0], "an unverified row must stay selectable"


def test_marker_spans_colour_only_the_unverified_marker_glyph():
    rows = [f"{MODEL_MARK} សុខភាព", f"{MODEL_MARK} សំបុក", "ខ្ញុំទៅ"]
    flags = [(True, True), (True, False), (False, True)]

    spans = marker_spans(rows, flags)

    # Only the red (unverified) row needs an attribute; verified white ✦ is the
    # default label colour and needs none.
    assert spans == [(1, UNVERIFIED_FG, 0, len(MODEL_MARK))]


def test_marker_spans_are_empty_without_model_candidates():
    assert marker_spans(["ខ្ញុំទៅ", "ខ្ញុំតៅ"], [(False, True), (False, True)]) == []


def test_marker_spans_tolerate_missing_flags():
    assert marker_spans([f"{MODEL_MARK} ស"], []) == []
