"""check_ipa.sh: an .ipa missing a file the store build must carry is refused before upload.

export_ipa.sh runs it on KHMERIME_IPA_REQUIRE (set in the git-ignored upload config), so a
build made the wrong way stops on this Mac instead of reaching App Store Connect.
"""

import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "platforms" / "ios" / "keyboard" / "check_ipa.sh"
APPEX = "KhmerIME.app/PlugIns/KhmerIMEKeyboard.appex"


def make_ipa(directory: Path, *files: str) -> Path:
    path = directory / "KhmerIME.ipa"
    with zipfile.ZipFile(path, "w") as z:
        z.writestr("Payload/KhmerIME.app/Info.plist", "")
        for name in files:
            z.writestr(f"Payload/{name}", "x")
    return path


def check(ipa: Path, *required: str) -> subprocess.CompletedProcess:
    return subprocess.run(["bash", str(SCRIPT), str(ipa), *required], capture_output=True, text=True)


class CheckIpaTest(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp())

    def test_passes_when_every_required_file_is_there(self):
        ipa = make_ipa(self.dir, f"{APPEX}/model.pack", f"{APPEX}/vocab.trie")
        result = check(ipa, f"{APPEX}/model.pack", f"{APPEX}/vocab.trie")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_refuses_an_ipa_missing_a_required_file(self):
        ipa = make_ipa(self.dir, f"{APPEX}/vocab.trie")
        result = check(ipa, f"{APPEX}/model.pack", f"{APPEX}/vocab.trie")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(f"{APPEX}/model.pack", result.stderr)
        self.assertNotIn(f"{APPEX}/vocab.trie is not", result.stderr)

    def test_the_same_name_elsewhere_does_not_count(self):
        ipa = make_ipa(self.dir, "KhmerIME.app/model.pack")
        result = check(ipa, f"{APPEX}/model.pack")
        self.assertNotEqual(result.returncode, 0)

    def test_nothing_required_passes(self):
        result = check(make_ipa(self.dir))
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_a_missing_ipa_is_an_error(self):
        result = check(self.dir / "absent.ipa", f"{APPEX}/model.pack")
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
