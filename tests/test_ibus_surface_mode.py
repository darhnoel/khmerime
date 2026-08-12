from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_candidate_renderer import FLAT, PHRASE, SEGMENT, surface_mode  # noqa: E402


def test_single_segment_composition_is_flat():
    assert surface_mode({"segmented_active": False, "segment_edit_active": False}) == FLAT


def test_segmented_session_without_edit_is_phrase_level():
    assert surface_mode({"segmented_active": True, "segment_edit_active": False}) == PHRASE


def test_segment_edit_mode_is_segment_level():
    assert surface_mode({"segmented_active": True, "segment_edit_active": True}) == SEGMENT


def test_segment_edit_wins_even_if_segmented_flag_is_absent():
    # Segment Edit Mode is only reachable from a Segmented Session, so an edit
    # flag without the session flag is a malformed snapshot. Honour the edit
    # flag rather than falling back to Flat, which would put roman back on the
    # rows in the one mode that most needs the room.
    assert surface_mode({"segment_edit_active": True}) == SEGMENT


def test_missing_flags_default_to_flat():
    assert surface_mode({}) == FLAT


def test_non_dict_snapshot_defaults_to_flat():
    assert surface_mode(None) == FLAT
