"""Phrase-level rows come from `phrase_candidates` (whole-composition
hypotheses), not from `candidates` (the focused segment's words). Mirrors the
macOS `CandidateSurface::from_snapshot` filter — linux-ibus ADR-0001.
"""

from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_candidate_renderer import phrase_rows  # noqa: E402


def _phrase(text: str, segments: int, from_model: bool = False) -> dict:
    return {
        "text": text,
        "segments": [{"output": text, "input": "x"} for _ in range(segments)],
        "from_model": from_model,
        "lexicon_verified": True,
    }


def test_multi_segment_phrases_become_rows():
    snapshot = {
        "phrase_candidates": [_phrase("ខ្ញុំទៅ", 2), _phrase("ខ្ញុំតៅ", 2)],
        "selected_phrase_index": 0,
    }

    rows, indices, selected = phrase_rows(snapshot)

    assert rows == ["ខ្ញុំទៅ", "ខ្ញុំតៅ"]
    assert indices == [0, 1]
    assert selected == 0


def test_single_word_flat_fallbacks_are_dropped():
    # A single-segment phrase is a first-word guess, not an alternative reading
    # of the whole composition. Showing it made the panel look wrong on macOS.
    snapshot = {
        "phrase_candidates": [_phrase("ខ្ញុំទៅ", 2), _phrase("ខ្ញុំ", 1), _phrase("ខ្ញុំតៅ", 2)],
        "selected_phrase_index": 0,
    }

    rows, indices, _ = phrase_rows(snapshot)

    assert rows == ["ខ្ញុំទៅ", "ខ្ញុំតៅ"]
    # Indices must stay session-relative so select_phrase targets the right one.
    assert indices == [0, 2]


def test_single_word_model_rescue_is_kept():
    # A one-word AI rescue spans the whole composition, so it IS a whole-phrase
    # reading despite having a single segment.
    snapshot = {
        "phrase_candidates": [_phrase("ខ្ញុំទៅ", 2), _phrase("សុខ", 1, from_model=True)],
        "selected_phrase_index": 0,
    }

    rows, indices, _ = phrase_rows(snapshot)

    # The rescue is kept, and carries the ✦ provenance marker (ADR-0016) so it
    # cannot pass as a human-reviewed Lexicon reading.
    assert rows == ["ខ្ញុំទៅ", "✦ សុខ"]
    assert indices == [0, 1]


def test_selected_index_is_mapped_to_the_visible_row():
    snapshot = {
        "phrase_candidates": [_phrase("ខ្ញុំទៅ", 2), _phrase("ខ្ញុំ", 1), _phrase("ខ្ញុំតៅ", 2)],
        "selected_phrase_index": 2,
    }

    rows, indices, selected = phrase_rows(snapshot)

    # Session index 2 is visible row 1 once the flat fallback is filtered out.
    assert indices == [0, 2]
    assert selected == 1
    assert rows[selected] == "ខ្ញុំតៅ"


def test_selected_index_pointing_at_a_filtered_row_has_no_visible_selection():
    snapshot = {
        "phrase_candidates": [_phrase("ខ្ញុំទៅ", 2), _phrase("ខ្ញុំ", 1)],
        "selected_phrase_index": 1,
    }

    _, _, selected = phrase_rows(snapshot)

    assert selected is None


def test_malformed_entries_are_skipped_not_fatal():
    snapshot = {
        "phrase_candidates": [_phrase("ខ្ញុំទៅ", 2), None, {"text": ""}, {"segments": []}],
        "selected_phrase_index": 0,
    }

    rows, indices, _ = phrase_rows(snapshot)

    assert rows == ["ខ្ញុំទៅ"]
    assert indices == [0]


def test_absent_phrase_candidates_yield_no_rows():
    rows, indices, selected = phrase_rows({})

    assert rows == []
    assert indices == []
    assert selected is None
