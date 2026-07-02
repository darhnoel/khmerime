"""Behavior tests for the IBus render planner.

The planner dedups UI updates while the user cycles the Candidate List:
identical Preedit updates are skipped and unchanged candidate rows are
re-rendered as a cursor-only move instead of a full table reload.
"""

from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_render_plan import RenderPlanner  # noqa: E402


def test_first_snapshot_renders_preedit_and_reloads_candidate_list():
    planner = RenderPlanner()

    assert planner.preedit_changed(("sa", None)) is True
    assert planner.candidate_list_action(["សា", "សារ"], 0) == "reload"


def test_identical_preedit_is_not_resent():
    planner = RenderPlanner()
    planner.preedit_changed(("sa", None))

    assert planner.preedit_changed(("sa", None)) is False

    # A different raw preedit (user typed another key) must render again.
    assert planner.preedit_changed(("sam", None)) is True


def test_space_cycling_same_rows_is_a_cursor_only_move():
    planner = RenderPlanner()
    rows = ["សា", "សារ", "ស្អា"]
    planner.candidate_list_action(rows, 0)

    # Space moved the selection; the rows themselves did not change.
    assert planner.candidate_list_action(rows, 1) == "cursor"
    assert planner.candidate_list_action(rows, 2) == "cursor"


def test_unchanged_rows_and_cursor_skip_the_candidate_list_update():
    planner = RenderPlanner()
    rows = ["សា", "សារ"]
    planner.candidate_list_action(rows, 0)

    # e.g. a refinement snapshot arrived but nothing visible moved.
    assert planner.candidate_list_action(rows, 0) == "skip"


def test_changed_rows_reload_even_if_cursor_is_unchanged():
    planner = RenderPlanner()
    planner.candidate_list_action(["សា", "សារ"], 0)

    # User typed another letter: new candidate rows, same cursor position.
    assert planner.candidate_list_action(["សាម", "សំ"], 0) == "reload"
    # And an empty list (composition ended) is also a rows change.
    assert planner.candidate_list_action([], 0) == "reload"


def test_reset_forces_full_render_of_identical_state():
    planner = RenderPlanner()
    rows = ["សា"]
    planner.preedit_changed(("sa", None))
    planner.candidate_list_action(rows, 0)

    # Commit/hide cleared the panel; identical state must be re-sent in full.
    planner.reset()

    assert planner.preedit_changed(("sa", None)) is True
    assert planner.candidate_list_action(rows, 0) == "reload"
