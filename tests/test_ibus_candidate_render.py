from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_candidate_render import candidate_rows  # noqa: E402


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
