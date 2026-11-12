#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import shutil
import os
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SOURCE_DIR = ROOT / "site" / "download"
OUT_DIR = Path(os.environ.get("KHMERIME_DOWNLOAD_PAGE_OUT", ROOT / "dist" / "download-page"))
CONFIG_FILE = SOURCE_DIR / "site.config.json"

# Stable installer names under <base>/latest/. The release upload keeps a copy of
# each installer under these names, so the page never needs rebuilding when a new
# installer is published.
LATEST_WINDOWS_FILE = "KhmerIME-windows-x64.msi"
LATEST_LINUX_FILE = "khmerime-linux-amd64.deb"
LATEST_MACOS_FILE = "KhmerIME-macos.pkg"

# Checksum manifest + GPG material published next to the installers under <base>/latest/.
SUMS_FILE = "SHA256SUMS"
SUMS_ASC_FILE = "SHA256SUMS.asc"
GPGKEY_FILE = "KhmerIME-signing-key.asc"


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
    macos_file = f"KhmerIME-{version}-macos.pkg"
    windows_src = Path(os.environ.get("KHMERIME_WINDOWS_MSI", ROOT / "dist" / "windows" / windows_file))
    linux_src = Path(os.environ.get("KHMERIME_LINUX_DEB", ROOT / "dist" / "linux" / linux_file))
    macos_src = Path(os.environ.get("KHMERIME_MACOS_PKG", ROOT / "dist" / "macos" / macos_file))
    # Set when installers are served from a separate download host; the page then
    # links to <base>/latest/<stable name> and carries no installers itself.
    download_base_url = os.environ.get("KHMERIME_DOWNLOAD_BASE_URL", "").rstrip("/")

    if download_base_url:
        latest = f"{download_base_url}/latest"
        windows_url = f"{latest}/{LATEST_WINDOWS_FILE}"
        linux_url = f"{latest}/{LATEST_LINUX_FILE}"
        macos_url = f"{latest}/{LATEST_MACOS_FILE}"
        sums_url = f"{latest}/{SUMS_FILE}"
        sums_asc_url = f"{latest}/{SUMS_ASC_FILE}"
        gpgkey_url = f"{latest}/{GPGKEY_FILE}"
    else:
        windows_url = f"downloads/windows/{windows_file}"
        linux_url = f"downloads/linux/{linux_file}"
        macos_url = f"downloads/macos/{macos_file}"
        sums_url = f"downloads/{SUMS_FILE}"
        sums_asc_url = f"downloads/{SUMS_ASC_FILE}"
        gpgkey_url = f"downloads/{GPGKEY_FILE}"

    # Download buttons stay hidden until installers are published somewhere public.
    downloads_enabled = os.environ.get("KHMERIME_DOWNLOADS_ENABLED", config.get("downloads_enabled", "true"))
    downloads = "on" if downloads_enabled.strip().lower() in ("1", "true", "yes", "on") else "off"

    OUT_DIR.mkdir(parents=True, exist_ok=True)

    # A short fingerprint of the demo videos, added to their URLs, so a browser
    # never keeps playing an old copy after they are re-rendered.
    demo_dir = SOURCE_DIR / "assets" / "demo"
    digest = hashlib.sha256()
    for demo_file in sorted(demo_dir.glob("*")) if demo_dir.is_dir() else []:
        digest.update(demo_file.read_bytes())
    demo_version = digest.hexdigest()[:10]

    tokens = {
        "@DEMO_V@": demo_version,
        "@DOWNLOADS@": downloads,
        "@IOS_APP_URL@": config.get("ios_app_url", "https://apps.apple.com/app/khmerime/id6781253596"),
        "@VERSION@": version,
        "@WINDOWS_FILE@": windows_file,
        "@LINUX_FILE@": linux_file,
        "@MACOS_FILE@": macos_file,
        "@WINDOWS_URL@": windows_url,
        "@LINUX_URL@": linux_url,
        "@MACOS_URL@": macos_url,
        # Stable (served) filenames the download saves as — used in verify commands.
        "@WINDOWS_STABLE@": LATEST_WINDOWS_FILE,
        "@LINUX_STABLE@": LATEST_LINUX_FILE,
        "@MACOS_STABLE@": LATEST_MACOS_FILE,
        # SHA-256 of each published installer (from site.config.json).
        "@WINDOWS_SHA@": config.get("checksum_windows", ""),
        "@LINUX_SHA@": config.get("checksum_linux", ""),
        "@MACOS_SHA@": config.get("checksum_macos", ""),
        "@SUMS_URL@": sums_url,
        "@SUMS_ASC_URL@": sums_asc_url,
        "@GPGKEY_URL@": gpgkey_url,
        "@ONLINE_BETA_URL@": online_beta_url,
    }
    copy_with_tokens(SOURCE_DIR / "index.html", OUT_DIR / "index.html", tokens)
    copy_with_tokens(SOURCE_DIR / "privacy.html", OUT_DIR / "privacy.html", tokens)
    shutil.copy2(SOURCE_DIR / "styles.css", OUT_DIR / "styles.css")
    copy_with_tokens(SOURCE_DIR / "download-detect.js", OUT_DIR / "download-detect.js", tokens)
    assets_dir = SOURCE_DIR / "assets"
    if assets_dir.is_dir():
        shutil.copytree(assets_dir, OUT_DIR / "assets", dirs_exist_ok=True)

    if download_base_url:
        print(f"[khmerime] Installer links point at {download_base_url}/latest/")
    else:
        (OUT_DIR / "downloads" / "windows").mkdir(parents=True, exist_ok=True)
        (OUT_DIR / "downloads" / "linux").mkdir(parents=True, exist_ok=True)
        (OUT_DIR / "downloads" / "macos").mkdir(parents=True, exist_ok=True)
        if windows_src.is_file():
            shutil.copy2(windows_src, OUT_DIR / "downloads" / "windows" / windows_file)
        else:
            print(f"[khmerime] Windows MSI not copied. Upload manually to: downloads/windows/{windows_file}")
        if linux_src.is_file():
            shutil.copy2(linux_src, OUT_DIR / "downloads" / "linux" / linux_file)
        else:
            print(f"[khmerime] Linux .deb not copied. Upload manually to: downloads/linux/{linux_file}")
        if macos_src.is_file():
            shutil.copy2(macos_src, OUT_DIR / "downloads" / "macos" / macos_file)
        else:
            print(f"[khmerime] macOS .pkg not copied. Upload manually to: downloads/macos/{macos_file}")

    print(f"[khmerime] Download page written to: {OUT_DIR}")


if __name__ == "__main__":
    main()
