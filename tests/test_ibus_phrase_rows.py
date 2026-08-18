"""Phrase-level rows come from `phrase_candidates` (whole-composition
hypotheses), not from `candidates` (the focused segment's words). Mirrors the
macOS `CandidateSurface::from_snapshot` filter — linux-ibus ADR-0001.
"""

from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_candidate_renderer import PHRASE, phrase_rows, surface_mode  # noqa: E402


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


def test_single_word_model_rescue_uses_phrase_surface_without_segmentation():
    # A successful one-word model winner intentionally collapses a prior split
    # Segmented Session. It must still use Phrase rendering so model provenance
    # remains visible as the load-bearing ✦ marker from ADR-0016.
    snapshot = {
        "segmented_active": False,
        "phrase_candidates": [
            _phrase("ដំរី", 1),
            _phrase("តម្រា", 1, from_model=True),
            _phrase("ដំរ៉ា", 1),
        ],
    }

    assert surface_mode(snapshot) == PHRASE
    rows, indices, _ = phrase_rows(snapshot)
    assert rows == ["ដំរី", "✦ តម្រា", "ដំរ៉ា"]
    assert indices == [0, 1, 2]


def test_phrase_rows_show_each_candidates_complete_roman_pair():
    snapshot = {
        "raw_preedit": "domra",
        "segmented_active": True,
        "phrase_candidates": [
            {
                "text": "ដុំរ៉ា",
                "segments": [
                    {"output": "ដុំ", "input": "dom"},
                    {"output": "រ៉ា", "input": "ra", "roman_hints": ["ra"]},
                ],
                "from_model": False,
                "lexicon_verified": True,
            },
            {
                "text": "តម្រា",
                "segments": [
                    {
                        "output": "តម្រា",
                        "input": "domra",
                        "roman_hints": ["tamrea", "tomrea"],
                    }
                ],
                "from_model": True,
                "lexicon_verified": True,
            },
            {
                "text": "ដំរី",
                "segments": [
                    {
                        "output": "ដំរី",
                        "input": "domra",
                        "roman_hints": ["damrey", "domrei", "domrey"],
                    }
                ],
                "from_model": False,
                "lexicon_verified": True,
            },
        ],
    }

    rows, indices, _ = phrase_rows(snapshot)

    assert rows == [
        "ដុំរ៉ា (dom · ra)",
        "✦ តម្រា (tamrea / tomrea)",
        "ដំរី (damrey / domrei / domrey)",
    ]
    assert indices == [0, 1, 2]


def test_phrase_rows_show_only_top_five_total_including_model_results():
    snapshot = {
        "segmented_active": True,
        "phrase_candidates": [
            _phrase("ក", 1),
            _phrase("ខ", 1, from_model=True),
            _phrase("គ", 1),
            _phrase("ឃ", 1),
            _phrase("ង", 1, from_model=True),
            _phrase("ច", 1),
            _phrase("ឆ", 1),
        ],
        "selected_phrase_index": 4,
    }

    rows, indices, selected = phrase_rows(snapshot)

    assert rows == ["ក", "✦ ខ", "គ", "ឃ", "✦ ង"]
    assert indices == [0, 1, 2, 3, 4]
    assert selected == 4


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


def test_a_truncated_model_rescue_keeps_the_last_visible_slot():
    # A Word Rescuer proposal can rank below the cap: it has no Lexicon frequency
    # prior (the word is not in the Lexicon — the reason the model was needed), so
    # ordinary readings out-score it. Worse, those single-segment readings are only
    # eligible BECAUSE a model candidate exists, so they would fill every slot and
    # evict the rescue that admitted them. Reserve the last row for it.
    snapshot = {
        "phrase_candidates": [_phrase("ស%d" % i, 1) for i in range(7)]
        + [_phrase("តម្រា", 1, from_model=True)],
        "selected_phrase_index": 0,
    }

    rows, indices, _ = phrase_rows(snapshot)

    assert len(rows) == 5, "the cap still holds — the rescue takes a slot, it does not add one"
    assert rows[-1] == "✦ តម្រា", "the rescue must be reachable, not truncated away"
    assert indices[-1] == 7, "the reserved row still maps to its own session index"
    assert rows[:4] == ["ស0", "ស1", "ស2", "ស3"], "decoder order is kept for the unreserved rows"


def test_a_rescue_already_inside_the_cap_is_not_moved():
    # The reservation is a rescue for truncation only. When the model row already
    # fits, decoder order must be left completely alone.
    snapshot = {
        "phrase_candidates": [
            _phrase("ក", 2),
            _phrase("តម្រា", 1, from_model=True),
            _phrase("ខ", 2),
            _phrase("គ", 2),
            _phrase("ឃ", 2),
            _phrase("ង", 2),
        ],
        "selected_phrase_index": 0,
    }

    rows, indices, _ = phrase_rows(snapshot)

    assert rows == ["ក", "✦ តម្រា", "ខ", "គ", "ឃ"]
    assert indices == [0, 1, 2, 3, 4]
