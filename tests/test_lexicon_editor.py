import csv
import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANAGER_DIR = ROOT / "scripts" / "data" / "lexicon"
if str(MANAGER_DIR) not in sys.path:
    sys.path.insert(0, str(MANAGER_DIR))
EDITOR_DIR = ROOT / "tools" / "lexicon-editor"
if str(EDITOR_DIR) not in sys.path:
    sys.path.insert(0, str(EDITOR_DIR))
MODULE_PATH = EDITOR_DIR / "server.py"
SPEC = importlib.util.spec_from_file_location("lexicon_editor_server", MODULE_PATH)
assert SPEC and SPEC.loader
server = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = server
SPEC.loader.exec_module(server)


def write_chunk(path: Path, rows: list[list[str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(server.chunks.CHUNK_COLUMNS)
        writer.writerows(rows)


def make_state(tmp_path: Path) -> "server.EditorState":
    chunks_dir = tmp_path / "chunks"
    write_chunk(
        chunks_dir / "chunk_0001.csv",
        [["srap", "ស្រាប់", "5", "km", "words", "approved", ""]],
    )
    return server.EditorState(
        root=tmp_path,
        chunks_dir=chunks_dir,
        runtime_path=tmp_path / "roman_lookup.csv",
    )


def make_freq_conflict_state(tmp_path: Path) -> "server.EditorState":
    chunks_dir = tmp_path / "chunks"
    write_chunk(
        chunks_dir / "chunk_0001.csv",
        [
            ["srap", "ស្រាប់", "5", "km", "words", "approved", ""],
            ["sraab", "ស្រាប់", "12", "km", "words", "approved", ""],
            ["thmei", "ថ្មី", "3", "km", "words", "approved", ""],
        ],
    )
    return server.EditorState(
        root=tmp_path,
        chunks_dir=chunks_dir,
        runtime_path=tmp_path / "roman_lookup.csv",
    )


class LexiconEditorFrequencyConflictTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_frequency_conflicts_groups_rows_by_target_with_differing_freq(self) -> None:
        state = make_freq_conflict_state(self.tmp_path)

        conflicts = state.frequency_conflicts()

        self.assertEqual(len(conflicts), 1)
        conflict = conflicts[0]
        self.assertEqual(conflict["target"], "ស្រាប់")
        self.assertEqual(conflict["freq_lang"], "km")
        self.assertEqual({r["freq"] for r in conflict["rows"]}, {"5", "12"})
        self.assertEqual({r["roman"] for r in conflict["rows"]}, {"srap", "sraab"})
        for row in conflict["rows"]:
            self.assertIn("id", row)

    def test_frequency_conflicts_empty_when_consistent(self) -> None:
        state = make_state(self.tmp_path)
        self.assertEqual(state.frequency_conflicts(), [])

    def test_bulk_edit_freq_resolves_a_conflict(self) -> None:
        state = make_freq_conflict_state(self.tmp_path)
        conflict = state.frequency_conflicts()[0]
        ids = [row["id"] for row in conflict["rows"]]

        state.api_bulk_edit({"row_ids": ids, "column": "freq", "value": "12"})

        self.assertEqual(state.frequency_conflicts(), [])

    def test_save_build_check_raises_with_conflict_detail(self) -> None:
        state = make_freq_conflict_state(self.tmp_path)
        # force a dirty draft so save takes the validating path
        row_id = state.current_rows()[0]["_id"]
        state.api_edit_cell({"row_id": row_id, "column": "notes", "value": "x"})

        with self.assertRaises(server.EditorError) as ctx:
            state.api_save_build_check()

        detail = getattr(ctx.exception, "detail", None)
        self.assertIsNotNone(detail)
        self.assertEqual(len(detail["frequency_conflicts"]), 1)
        self.assertEqual(detail["frequency_conflicts"][0]["target"], "ស្រាប់")

    def test_bulk_edit_freq_rejects_non_positive(self) -> None:
        state = make_freq_conflict_state(self.tmp_path)
        ids = [row["id"] for row in state.frequency_conflicts()[0]["rows"]]

        with self.assertRaisesRegex(server.EditorError, "positive integer"):
            state.api_bulk_edit({"row_ids": ids, "column": "freq", "value": "0"})


def make_duplicate_key_state(tmp_path: Path) -> "server.EditorState":
    chunks_dir = tmp_path / "chunks"
    write_chunk(
        chunks_dir / "chunk_0001.csv",
        [["amara", "អមរ", "5", "km", "words", "approved", ""]],
    )
    write_chunk(
        chunks_dir / "chunk_tonle_more.csv",
        [["amara", "អមរ", "1", "km", "words", "approved", ""]],
    )
    return server.EditorState(
        root=tmp_path,
        chunks_dir=chunks_dir,
        runtime_path=tmp_path / "roman_lookup.csv",
    )


class LexiconEditorDuplicateKeyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_duplicate_key_conflicts_groups_approved_rows_across_chunks(self) -> None:
        state = make_duplicate_key_state(self.tmp_path)

        conflicts = state.duplicate_key_conflicts()

        self.assertEqual(len(conflicts), 1)
        conflict = conflicts[0]
        self.assertEqual((conflict["roman"], conflict["target"], conflict["freq_lang"]), ("amara", "អមរ", "km"))
        self.assertEqual({r["chunk"] for r in conflict["rows"]}, {"chunk_0001.csv", "chunk_tonle_more.csv"})
        for row in conflict["rows"]:
            self.assertIn("id", row)

    def test_duplicate_key_conflicts_empty_when_unique(self) -> None:
        state = make_state(self.tmp_path)
        self.assertEqual(state.duplicate_key_conflicts(), [])

    def test_save_build_check_raises_with_duplicate_detail(self) -> None:
        state = make_duplicate_key_state(self.tmp_path)
        row_id = state.current_rows()[0]["_id"]
        state.api_edit_cell({"row_id": row_id, "column": "notes", "value": "x"})

        with self.assertRaises(server.EditorError) as ctx:
            state.api_save_build_check()

        detail = getattr(ctx.exception, "detail", None)
        self.assertIsNotNone(detail)
        self.assertEqual(len(detail["duplicate_keys"]), 1)
        self.assertEqual(detail["duplicate_keys"][0]["roman"], "amara")

    def test_disabling_one_row_resolves_the_duplicate(self) -> None:
        state = make_duplicate_key_state(self.tmp_path)
        rac_row = next(r for r in state.duplicate_key_conflicts()[0]["rows"] if r["chunk"] == "chunk_tonle_more.csv")

        state.api_bulk_edit({"row_ids": [rac_row["id"]], "column": "status", "value": "disabled"})

        self.assertEqual(state.duplicate_key_conflicts(), [])


def make_three_chunk_state(tmp_path: Path) -> "server.EditorState":
    chunks_dir = tmp_path / "chunks"
    write_chunk(chunks_dir / "chunk_0001.csv", [["a", "ក", "1", "km", "words", "approved", ""]])
    write_chunk(chunks_dir / "chunk_0002.csv", [["b", "ខ", "1", "km", "words", "approved", ""]])
    write_chunk(chunks_dir / "chunk_0003.csv", [["c", "គ", "1", "km", "words", "approved", ""]])
    return server.EditorState(
        root=tmp_path,
        chunks_dir=chunks_dir,
        runtime_path=tmp_path / "roman_lookup.csv",
    )


class LexiconEditorMultiChunkFilterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_api_rows_filters_to_selected_chunk_set(self) -> None:
        state = make_three_chunk_state(self.tmp_path)

        result = state.api_rows({"chunk": ["chunk_0001.csv", "chunk_0003.csv"]})

        chunks_shown = {row["chunk"] for row in result["data"]}
        self.assertEqual(chunks_shown, {"chunk_0001.csv", "chunk_0003.csv"})
        self.assertEqual(result["total"], 2)

    def test_api_rows_no_chunk_shows_all(self) -> None:
        state = make_three_chunk_state(self.tmp_path)
        result = state.api_rows({})
        self.assertEqual(result["total"], 3)

    def test_api_rows_filters_by_multiple_statuses(self) -> None:
        chunks_dir = self.tmp_path / "chunks"
        write_chunk(
            chunks_dir / "chunk_0001.csv",
            [
                ["a", "ក", "1", "km", "words", "approved", ""],
                ["b", "ខ", "1", "km", "words", "draft", ""],
                ["c", "គ", "1", "km", "words", "disabled", ""],
            ],
        )
        state = server.EditorState(
            root=self.tmp_path, chunks_dir=chunks_dir, runtime_path=self.tmp_path / "roman_lookup.csv"
        )

        result = state.api_rows({"status": ["approved", "draft"]})

        self.assertEqual({row["status"] for row in result["data"]}, {"approved", "draft"})
        self.assertEqual(result["total"], 2)


class LexiconEditorTargetRegexFilterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def _state(self) -> "server.EditorState":
        chunks_dir = self.tmp_path / "chunks"
        write_chunk(
            chunks_dir / "chunk_0001.csv",
            [
                ["srau", "ស្រៅ", "1", "km", "words", "approved", ""],
                ["srav", "ស្រៈ", "1", "km", "words", "approved", ""],
                ["thmei", "ថ្មី", "1", "km", "words", "approved", ""],
            ],
        )
        return server.EditorState(
            root=self.tmp_path, chunks_dir=chunks_dir, runtime_path=self.tmp_path / "roman_lookup.csv"
        )

    def test_filters_targets_by_regex_suffix(self) -> None:
        result = self._state().api_rows({"target": ["ៅ$"]})
        self.assertEqual({row["roman"] for row in result["data"]}, {"srau"})

    def test_regex_matches_only_target_not_roman(self) -> None:
        # 'srau' is roman-only; a target regex must not match roman text.
        result = self._state().api_rows({"target": ["srau"]})
        self.assertEqual(result["total"], 0)

    def test_ands_with_search_query(self) -> None:
        result = self._state().api_rows({"target": ["ស"], "query": ["thmei"]})
        self.assertEqual(result["total"], 0)

    def test_invalid_regex_shows_all(self) -> None:
        result = self._state().api_rows({"target": ["["]})
        self.assertEqual(result["total"], 3)

    def test_empty_target_shows_all(self) -> None:
        result = self._state().api_rows({"target": [""]})
        self.assertEqual(result["total"], 3)


class LexiconEditorRuntimeFilterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def _state(self) -> "server.EditorState":
        chunks_dir = self.tmp_path / "chunks"
        write_chunk(
            chunks_dir / "chunk_0001.csv",
            [
                ["a", "ក", "1", "km", "words", "approved", ""],
                ["b", "ខ", "1", "km", "words", "draft", ""],
                ["c", "គ", "1", "km", "words", "disabled", ""],
            ],
        )
        return server.EditorState(
            root=self.tmp_path, chunks_dir=chunks_dir, runtime_path=self.tmp_path / "roman_lookup.csv"
        )

    def test_included_shows_only_approved(self) -> None:
        result = self._state().api_rows({"runtime": ["included"]})
        self.assertEqual({row["roman"] for row in result["data"]}, {"a"})

    def test_excluded_shows_non_approved(self) -> None:
        result = self._state().api_rows({"runtime": ["excluded"]})
        self.assertEqual({row["roman"] for row in result["data"]}, {"b", "c"})

    def test_all_or_empty_shows_everything(self) -> None:
        self.assertEqual(self._state().api_rows({"runtime": ["all"]})["total"], 3)
        self.assertEqual(self._state().api_rows({"runtime": [""]})["total"], 3)


class LexiconEditorReviewBadgeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_disabled_rows_count_in_total_but_not_actionable(self) -> None:
        chunks_dir = self.tmp_path / "chunks"
        write_chunk(
            chunks_dir / "chunk_0001.csv",
            [
                ["a", "ក", "1", "km", "words", "approved", ""],
                ["b", "ខ", "1", "km", "words", "disabled", ""],
            ],
        )
        state = server.EditorState(
            root=self.tmp_path, chunks_dir=chunks_dir, runtime_path=self.tmp_path / "roman_lookup.csv"
        )

        result = state.api_problems()

        self.assertEqual(result["total"], 1)  # the disabled row is listed
        self.assertEqual(result["actionable"], 0)  # but not counted as actionable


class LexiconEditorBootstrapTests(unittest.TestCase):
    def test_root_resolves_to_repo_and_manager_is_importable(self) -> None:
        # Guards against the file-depth regression: state.py must resolve ROOT
        # to the repo root so it can import manage_lexicon_chunks at startup.
        self.assertTrue((server.EditorState.__module__,))
        from editor import state  # noqa: PLC0415

        self.assertEqual(state.ROOT, ROOT)
        self.assertTrue((state.MANAGER_DIR / "manage_lexicon_chunks.py").exists())


class LexiconEditorStateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_edited_row_count_tracks_edits_adds_and_deletes(self) -> None:
        state = make_state(self.tmp_path)
        self.assertEqual(state.api_meta()["edited_rows"], 0)

        row_id = state.current_rows()[0]["_id"]
        state.api_edit_cell({"row_id": row_id, "column": "roman", "value": "sraap"})
        self.assertEqual(state.api_meta()["edited_rows"], 1)

        state.api_add_row({"after_row_id": row_id})
        self.assertEqual(state.api_meta()["edited_rows"], 2)

        state.api_delete_rows({"row_ids": [row_id]})
        self.assertEqual(state.api_meta()["edited_rows"], 2)

    def test_edit_cell_marks_chunk_dirty(self) -> None:
        state = make_state(self.tmp_path)
        row_id = state.current_rows()[0]["_id"]

        state.api_edit_cell({"row_id": row_id, "column": "roman", "value": "sraap"})

        self.assertIn("chunk_0001.csv", state.dirty_chunks)
        self.assertEqual(state.current_rows()[0]["roman"], "sraap")

    def test_edit_cell_returns_dirty_flag_for_client_auto_select(self) -> None:
        # Contract the client relies on to auto-select edited rows: a real
        # change returns dirty=True; committing a cell to its unchanged value
        # returns dirty=False (so tabbing through cells won't select the row).
        state = make_state(self.tmp_path)
        row = state.current_rows()[0]
        row_id, current = row["_id"], row["roman"]

        noop = state.api_edit_cell({"row_id": row_id, "column": "roman", "value": current})
        self.assertFalse(noop["row"]["dirty"])

        changed = state.api_edit_cell({"row_id": row_id, "column": "roman", "value": "sraap"})
        self.assertTrue(changed["row"]["dirty"])

    def test_undo_reverts_the_last_edit(self) -> None:
        state = make_state(self.tmp_path)
        row_id = state.current_rows()[0]["_id"]
        state.api_edit_cell({"row_id": row_id, "column": "roman", "value": "sraap"})

        state.api_undo()

        self.assertEqual(state.current_rows()[0]["roman"], "srap")
        self.assertNotIn("chunk_0001.csv", state.dirty_chunks)

    def test_soft_remove_disables_the_row(self) -> None:
        state = make_state(self.tmp_path)
        row_id = state.current_rows()[0]["_id"]

        state.api_soft_remove({"row_ids": [row_id]})

        row = state.current_rows()[0]
        self.assertEqual(row["status"], "disabled")
        self.assertIn("disabled in lexicon editor", row["notes"])

    def test_save_writes_chunk_and_builds_runtime_from_approved_rows(self) -> None:
        state = make_state(self.tmp_path)
        row_id = state.current_rows()[0]["_id"]
        state.api_edit_cell({"row_id": row_id, "column": "roman", "value": "sraap"})

        state.api_save_build_check()

        self.assertEqual(state.dirty_chunks, set())
        written = (state.chunks_dir / "chunk_0001.csv").read_text(encoding="utf-8")
        self.assertIn("sraap", written)
        runtime = state.runtime_path.read_text(encoding="utf-8")
        self.assertIn("sraap,ស្រាប់,5,km", runtime)

    def test_delete_rows_removes_the_row_entirely(self) -> None:
        state = make_state(self.tmp_path)
        row_id = state.current_rows()[0]["_id"]

        state.api_delete_rows({"row_ids": [row_id]})

        self.assertEqual(state.current_rows(), [])
        self.assertIn("chunk_0001.csv", state.dirty_chunks)

    def test_delete_rows_is_undoable(self) -> None:
        state = make_state(self.tmp_path)
        row_id = state.current_rows()[0]["_id"]
        state.api_delete_rows({"row_ids": [row_id]})

        state.api_undo()

        self.assertEqual(len(state.current_rows()), 1)
        self.assertEqual(state.current_rows()[0]["roman"], "srap")


def make_multi_row_state(tmp_path: Path) -> "server.EditorState":
    chunks_dir = tmp_path / "chunks"
    write_chunk(
        chunks_dir / "chunk_0001.csv",
        [
            ["aaxxxx", "ក", "1", "km", "words", "approved", ""],
            ["aayyyy", "ខ", "1", "km", "words", "approved", ""],
            ["bbzzzz", "គ", "1", "km", "words", "approved", ""],
        ],
    )
    return server.EditorState(
        root=tmp_path,
        chunks_dir=chunks_dir,
        runtime_path=tmp_path / "roman_lookup.csv",
    )


class LexiconEditorBulkRegexTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_preview_shows_old_to_new_without_mutating(self) -> None:
        state = make_multi_row_state(self.tmp_path)
        ids = [row["_id"] for row in state.current_rows()]

        result = state.api_bulk_regex_preview(
            {"row_ids": ids, "column": "roman", "pattern": "^a", "replacement": ""}
        )

        changes = {change["old"]: change["new"] for change in result["changes"]}
        self.assertEqual(changes, {"aaxxxx": "axxxx", "aayyyy": "ayyyy"})
        self.assertNotIn("chunk_0001.csv", state.dirty_chunks)
        self.assertEqual([row["roman"] for row in state.current_rows()][0], "aaxxxx")

    def test_apply_mutates_selected_rows_and_is_undoable(self) -> None:
        state = make_multi_row_state(self.tmp_path)
        ids = [row["_id"] for row in state.current_rows()]

        state.api_bulk_regex_apply(
            {"row_ids": ids, "column": "roman", "pattern": "^a", "replacement": ""}
        )

        self.assertEqual([row["roman"] for row in state.current_rows()], ["axxxx", "ayyyy", "bbzzzz"])
        self.assertIn("chunk_0001.csv", state.dirty_chunks)

        state.api_undo()
        self.assertEqual([row["roman"] for row in state.current_rows()], ["aaxxxx", "aayyyy", "bbzzzz"])

    def test_invalid_pattern_raises_before_any_change(self) -> None:
        state = make_multi_row_state(self.tmp_path)
        ids = [row["_id"] for row in state.current_rows()]

        with self.assertRaisesRegex(server.EditorError, "invalid pattern"):
            state.api_bulk_regex_apply(
                {"row_ids": ids, "column": "roman", "pattern": "(", "replacement": ""}
            )
        self.assertEqual(state.dirty_chunks, set())

    def test_no_row_ids_applies_to_filtered_set(self) -> None:
        # No explicit selection: regex should act on every row matching the filter.
        state = make_multi_row_state(self.tmp_path)

        result = state.api_bulk_regex_preview(
            {"filter": {"query": "aax"}, "column": "roman", "pattern": "^a", "replacement": ""}
        )

        changes = {change["old"]: change["new"] for change in result["changes"]}
        self.assertEqual(changes, {"aaxxxx": "axxxx"})

    def test_no_row_ids_no_filter_applies_to_all_rows(self) -> None:
        state = make_multi_row_state(self.tmp_path)

        result = state.api_bulk_regex_preview(
            {"filter": {}, "column": "roman", "pattern": "^a", "replacement": ""}
        )

        changes = {change["old"]: change["new"] for change in result["changes"]}
        self.assertEqual(changes, {"aaxxxx": "axxxx", "aayyyy": "ayyyy"})

    def test_row_ids_win_over_filter(self) -> None:
        state = make_multi_row_state(self.tmp_path)
        first_id = state.current_rows()[0]["_id"]  # aaxxxx

        result = state.api_bulk_regex_preview(
            {"row_ids": [first_id], "filter": {}, "column": "roman", "pattern": "^a", "replacement": ""}
        )

        changes = {change["old"]: change["new"] for change in result["changes"]}
        self.assertEqual(changes, {"aaxxxx": "axxxx"})


def make_rules_state(tmp_path: Path) -> "server.EditorState":
    chunks_dir = tmp_path / "chunks"
    write_chunk(
        chunks_dir / "chunk_0001.csv",
        [
            ["tteak", "ក", "1", "km", "words", "approved", ""],  # overlap: tteak vs teak
            ["veak", "ខ", "1", "km", "words", "approved", ""],
            ["domros", "គ", "1", "km", "words", "approved", ""],  # ros$ -> ruos
            ["plain", "ឃ", "1", "km", "words", "approved", ""],   # untouched
        ],
    )
    return server.EditorState(
        root=tmp_path,
        chunks_dir=chunks_dir,
        runtime_path=tmp_path / "roman_lookup.csv",
    )


class LexiconEditorRegexRulesTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    RULES = [
        {"pattern": "tteak", "replacement": "tta"},
        {"pattern": "veak", "replacement": "va"},
        {"pattern": "teak", "replacement": "ta"},
        {"pattern": "ros$", "replacement": "ruos"},
    ]

    def test_apply_ordered_rules_resolves_overlap_and_chains(self) -> None:
        state = make_rules_state(self.tmp_path)
        ids = [row["_id"] for row in state.current_rows()]

        result = state.api_bulk_regex_rules_apply(
            {"row_ids": ids, "column": "roman", "rules": self.RULES}
        )

        romans = [row["roman"] for row in state.current_rows()]
        # tteak -> tta (rule 1 wins because it's ordered before teak)
        # veak -> va, domros -> domruos, plain untouched
        self.assertEqual(romans, ["tta", "va", "domruos", "plain"])
        self.assertEqual(result["updated"], 3)

    def test_rules_apply_is_one_undo_step(self) -> None:
        state = make_rules_state(self.tmp_path)
        ids = [row["_id"] for row in state.current_rows()]
        state.api_bulk_regex_rules_apply({"row_ids": ids, "column": "roman", "rules": self.RULES})

        state.api_undo()

        self.assertEqual(
            [row["roman"] for row in state.current_rows()], ["tteak", "veak", "domros", "plain"]
        )

    def test_rules_preview_shows_final_without_mutating(self) -> None:
        state = make_rules_state(self.tmp_path)
        ids = [row["_id"] for row in state.current_rows()]

        result = state.api_bulk_regex_rules_preview(
            {"row_ids": ids, "column": "roman", "rules": self.RULES}
        )

        changes = {c["old"]: c["new"] for c in result["changes"]}
        self.assertEqual(changes, {"tteak": "tta", "veak": "va", "domros": "domruos"})
        self.assertEqual(state.dirty_chunks, set())


if __name__ == "__main__":
    unittest.main()
