from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_candidate_renderer import FLAT, PHRASE, SEGMENT, candidate_rows  # noqa: E402


def test_candidate_rows_hide_ascii_raw_fallback_without_display_metadata():
    assert candidate_rows(["សាលារៀន", "salarien"], []) == ["សាលារៀន"]


def test_candidate_rows_render_recommended_hints_and_derived_marker():
    rows = candidate_rows(
        ["នេះ", "raw", "បកប្រែ"],
        [
            {"output": "នេះ", "recommended": True, "roman_hints": ["nih", "nis"]},
            {"output": "raw", "recommended": False, "roman_hints": []},
            {"output": "បកប្រែ", "recommended": False, "roman_hints": []},
        ],
    )

    assert rows == ["✓ នេះ (nih / nis)", "≈ បកប្រែ"]


def test_candidate_rows_falls_back_to_non_ascii_candidate_for_invalid_metadata():
    assert candidate_rows(["ទៅ", "tov"], [None, None]) == ["ទៅ"]


def test_candidate_rows_show_flagged_raw_fallback_without_marker():
    rows = candidate_rows(
        ["សាលារៀន", "salarien"],
        [
            {"output": "សាលារៀន", "recommended": True, "roman_hints": ["salarien"]},
            {"output": "salarien", "recommended": False, "roman_hints": [], "is_raw_fallback": True},
        ],
    )

    # The flagged ASCII roman fallback is admitted (unlike an unflagged ASCII
    # row) and rendered plainly, with no recommended/derived marker.
    assert rows == ["✓ សាលារៀន (salarien)", "salarien"]


# --- Candidate Surface modes -------------------------------------------------
#
# In Phrase and Segment mode the roman lives in the segment preview (the aux
# text header), so repeating it on every row wastes the lookup-table width and
# tells the user nothing new. Flat mode has no header, so the row keeps it.


def _display(output: str, hints: list[str], recommended: bool = False) -> dict:
    return {"output": output, "recommended": recommended, "roman_hints": hints}


def test_phrase_mode_rows_are_khmer_only():
    rows = candidate_rows(
        ["ខ្ញុំទៅ", "ខ្ញុំតៅ"],
        [_display("ខ្ញុំទៅ", ["nhomtov"], recommended=True), _display("ខ្ញុំតៅ", ["nhomtov"])],
        mode=PHRASE,
    )

    assert rows == ["✓ ខ្ញុំទៅ", "ខ្ញុំតៅ"]


def test_segment_mode_rows_are_khmer_only():
    rows = candidate_rows(
        ["សាលា", "សាឡា"],
        [_display("សាលា", ["sala"], recommended=True), _display("សាឡា", ["sala"])],
        mode=SEGMENT,
    )

    assert rows == ["✓ សាលា", "សាឡា"]


def test_flat_mode_keeps_the_per_row_roman_hint():
    rows = candidate_rows(
        ["សាលា"],
        [_display("សាលា", ["sala", "salaa"], recommended=True)],
        mode=FLAT,
    )

    assert rows == ["✓ សាលា (sala / salaa)"]


def test_mode_defaults_to_flat_when_the_caller_omits_it():
    rows = candidate_rows(["សាលា"], [_display("សាលា", ["sala"], recommended=True)])

    assert rows == ["✓ សាលា (sala)"]


def test_derived_marker_survives_in_phrase_mode():
    # `≈` marks a candidate with no roman hints. Dropping hints for display must
    # not turn a hinted row into a derived-looking one: the marker is chosen
    # from the underlying data, not from what the row ended up showing.
    rows = candidate_rows(
        ["ខ្ញុំទៅ", "បកប្រែ"],
        [_display("ខ្ញុំទៅ", ["nhomtov"]), _display("បកប្រែ", [])],
        mode=PHRASE,
    )

    assert rows == ["ខ្ញុំទៅ", "≈ បកប្រែ"]


def test_raw_fallback_stays_plain_in_every_mode():
    for mode in (FLAT, PHRASE, SEGMENT):
        rows = candidate_rows(
            ["សាលារៀន", "salarien"],
            [
                _display("សាលារៀន", ["salarien"], recommended=True),
                {"output": "salarien", "recommended": False, "roman_hints": [], "is_raw_fallback": True},
            ],
            mode=mode,
        )

        assert rows[-1] == "salarien", mode
