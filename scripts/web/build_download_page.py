#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import shutil
import os
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SOURCE_DIR = ROOT / "site" / "download"
OUT_DIR = ROOT / "dist" / "download-page"
CONFIG_FILE = SOURCE_DIR / "site.config.json"


def package_version() -> str:
    cargo_toml = ROOT / "Cargo.toml"
    match = re.search(r'(?m)^\s*version\s*=\s*"([^"]+)"', cargo_toml.read_text(encoding="utf-8"))
    if not match:
        raise SystemExit(f"Could not read package version from {cargo_toml}")
    return match.group(1)


def site_config() -> dict[str, str]:
    if not CONFIG_FILE.is_file():
        return {}
    try:
        config = json.loads(CONFIG_FILE.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise SystemExit(f"Invalid JSON in {CONFIG_FILE}: {error}") from error
    if not isinstance(config, dict):
        raise SystemExit(f"{CONFIG_FILE} must contain a JSON object")
    return {str(key): str(value) for key, value in config.items()}


def copy_with_tokens(source: Path, destination: Path, tokens: dict[str, str]) -> None:
    text = source.read_text(encoding="utf-8")
    for key, value in tokens.items():
        text = text.replace(key, value)
    # newline="\n" keeps LF endings; passed to open() (not write_text) so this
    # works on Python 3.9, where Path.write_text() lacks the newline kwarg.
    with open(destination, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)


def main() -> None:
    config = site_config()
    version = package_version()
    windows_file = f"KhmerIME-{version}-x64.msi"
    linux_file = f"khmerime_{version}_amd64.deb"
    online_beta_url = os.environ.get(
        "KHMERIME_ONLINE_URL",
        config.get("online_beta_url", "/khmerime-beta/"),
    )
    windows_src = Path(os.environ.get("KHMERIME_WINDOWS_MSI", ROOT / "dist" / "windows" / windows_file))
    linux_src = Path(os.environ.get("KHMERIME_LINUX_DEB", ROOT / "dist" / "linux" / linux_file))

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / "downloads" / "windows").mkdir(parents=True, exist_ok=True)
    (OUT_DIR / "downloads" / "linux").mkdir(parents=True, exist_ok=True)

    tokens = {
        "@VERSION@": version,
        "@WINDOWS_FILE@": windows_file,
        "@LINUX_FILE@": linux_file,
        "@ONLINE_BETA_URL@": online_beta_url,
    }
    copy_with_tokens(SOURCE_DIR / "index.html", OUT_DIR / "index.html", tokens)
    shutil.copy2(SOURCE_DIR / "styles.css", OUT_DIR / "styles.css")
    shutil.copy2(SOURCE_DIR / "download-detect.js", OUT_DIR / "download-detect.js")
    assets_dir = SOURCE_DIR / "assets"
    if assets_dir.is_dir():
        shutil.copytree(assets_dir, OUT_DIR / "assets", dirs_exist_ok=True)
    if windows_src.is_file():
        shutil.copy2(windows_src, OUT_DIR / "downloads" / "windows" / windows_file)
    else:
        print(f"[khmerime] Windows MSI not copied. Upload manually to: downloads/windows/{windows_file}")
    if linux_src.is_file():
        shutil.copy2(linux_src, OUT_DIR / "downloads" / "linux" / linux_file)
    else:
        print(f"[khmerime] Linux .deb not copied. Upload manually to: downloads/linux/{linux_file}")

    print(f"[khmerime] Download page written to: {OUT_DIR}")
    print("[khmerime] Upload the contents of this folder to cPanel.")


if __name__ == "__main__":
    main()
