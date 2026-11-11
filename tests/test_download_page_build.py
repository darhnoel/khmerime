import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "web" / "build_download_page.py"


def cargo_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    return re.search(r'(?m)^\s*version\s*=\s*"([^"]+)"', text).group(1)


def build(out_dir: Path, **env_overrides: str) -> str:
    env = {k: v for k, v in os.environ.items() if not k.startswith("KHMERIME_")}
    env["KHMERIME_DOWNLOAD_PAGE_OUT"] = str(out_dir)
    # Point installer sources at paths that do not exist, so no real dist/ file is copied.
    env["KHMERIME_WINDOWS_MSI"] = str(out_dir / "missing.msi")
    env["KHMERIME_LINUX_DEB"] = str(out_dir / "missing.deb")
    env.update(env_overrides)
    subprocess.run([sys.executable, str(SCRIPT)], check=True, env=env, capture_output=True, text=True)
    return (out_dir / "index.html").read_text(encoding="utf-8")


class DownloadPageBuildTest(unittest.TestCase):
    def test_no_template_tokens_left(self):
        with tempfile.TemporaryDirectory() as tmp:
            html = build(Path(tmp) / "site")
        self.assertEqual(re.findall(r"@[A-Z_]+@", html), [])

    def test_privacy_policy_is_published(self):
        # The App Store and Play Console require a public privacy-policy URL.
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "site"
            build(out)
            self.assertTrue((out / "privacy.html").is_file())

    def test_default_links_point_at_copied_installers(self):
        version = cargo_version()
        with tempfile.TemporaryDirectory() as tmp:
            html = build(Path(tmp) / "site")
        self.assertIn(f'href="downloads/windows/KhmerIME-{version}-x64.msi"', html)
        self.assertIn(f'href="downloads/linux/khmerime_{version}_amd64.deb"', html)

    def test_ios_links_to_the_app_store(self):
        with tempfile.TemporaryDirectory() as tmp:
            html = build(Path(tmp) / "site")
        self.assertIn('href="https://apps.apple.com/app/khmerime/id6781253596"', html)

    def test_downloads_switch_reaches_the_page(self):
        with tempfile.TemporaryDirectory() as tmp:
            off = build(Path(tmp) / "off", KHMERIME_DOWNLOADS_ENABLED="false")
            on = build(Path(tmp) / "on", KHMERIME_DOWNLOADS_ENABLED="true")
        self.assertIn('data-downloads="off"', off)
        self.assertIn('data-downloads="on"', on)

    def test_download_base_url_links_to_stable_latest_objects(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "site"
            html = build(out, KHMERIME_DOWNLOAD_BASE_URL="https://downloads.example.org/")
            self.assertIn('href="https://downloads.example.org/latest/KhmerIME-windows-x64.msi"', html)
            self.assertIn('href="https://downloads.example.org/latest/khmerime-linux-amd64.deb"', html)
            # Installers live on the download host, so the static site must not carry them.
            self.assertFalse((out / "downloads").exists())


if __name__ == "__main__":
    unittest.main()
