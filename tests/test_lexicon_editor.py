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


if __name__ == "__main__":
    unittest.main()
