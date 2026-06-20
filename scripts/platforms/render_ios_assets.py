#!/usr/bin/env python3
"""Generate iOS asset-catalog rasters from checked-in sources."""

from __future__ import annotations

import binascii
import shutil
import struct
import subprocess
import sys
import zlib
from pathlib import Path

from playwright.sync_api import sync_playwright


ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "adapters/ios-keyboard/swift/KhmerIME/Assets.xcassets"
DOWNLOAD_PAGE = ROOT / "site/download/index.html"
LOGO_PNG = ASSETS / "LogoCard.imageset/logocard.png"
APPICON_PNG = ASSETS / "AppIcon.appiconset/appicon-1024.png"
ABA_SOURCE = ROOT / "site/download/assets/my-aba-cropped.png"
ABA_DEST = ASSETS / "ABAQR.imageset/aba.png"


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def main() -> int:
    render_logo_card()
    render_appicon()
    copy_aba_qr()
    return 0


def render_logo_card() -> None:
    render_download_logo(LOGO_PNG, device_scale_factor=4)
    verify_square_png(LOGO_PNG, "LogoCard")
    print(f"generated {LOGO_PNG.relative_to(ROOT)}")


def render_appicon() -> None:
    render_download_logo(APPICON_PNG, device_scale_factor=1024 / 152)
    verify_appicon(APPICON_PNG)
    print(f"generated {APPICON_PNG.relative_to(ROOT)}")


def render_download_logo(dest: Path, device_scale_factor: float) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp_png = dest.with_suffix(".tmp.png")
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        context = browser.new_context(
            java_script_enabled=False,
            viewport={"width": 720, "height": 720},
            device_scale_factor=device_scale_factor,
        )
        page = context.new_page()
        page.goto(DOWNLOAD_PAGE.as_uri())
        page.locator(".logo-card").screenshot(path=tmp_png, omit_background=False)
        browser.close()

    strip_png_alpha(tmp_png, dest)
    tmp_png.unlink(missing_ok=True)


def copy_aba_qr() -> None:
    ABA_DEST.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ABA_SOURCE, ABA_DEST)
    print(f"generated {ABA_DEST.relative_to(ROOT)}")


def verify_appicon(path: Path) -> None:
    output = image_metadata(path)
    if "pixelWidth: 1024" not in output or "pixelHeight: 1024" not in output:
        raise RuntimeError(f"{path} must be 1024x1024:\n{output}")
    if "hasAlpha: no" not in output:
        raise RuntimeError(f"{path} must not contain an alpha channel:\n{output}")


def verify_square_png(path: Path, label: str) -> None:
    output = image_metadata(path)
    width = metadata_value(output, "pixelWidth")
    height = metadata_value(output, "pixelHeight")
    if width is None or height is None or width != height:
        raise RuntimeError(f"{label} must be square:\n{output}")
    if "hasAlpha: no" not in output:
        raise RuntimeError(f"{label} must not contain an alpha channel:\n{output}")


def image_metadata(path: Path) -> str:
    result = subprocess.run(
        ["sips", "-g", "pixelWidth", "-g", "pixelHeight", "-g", "hasAlpha", str(path)],
        check=True,
        text=True,
        capture_output=True,
    )
    return result.stdout


def metadata_value(output: str, key: str) -> int | None:
    marker = f"{key}: "
    for line in output.splitlines():
        line = line.strip()
        if line.startswith(marker):
            return int(line.removeprefix(marker))
    return None


def strip_png_alpha(source: Path, dest: Path) -> None:
    width, height, color_type, rows = read_png_rows(source)
    if color_type == 2:
        write_rgb_png(dest, width, height, rows)
        return
    if color_type != 6:
        raise RuntimeError(f"unsupported PNG color type {color_type}; expected RGB or RGBA")

    rgb_rows = []
    for row in rows:
        rgb = bytearray()
        for index in range(0, len(row), 4):
            rgb.extend(row[index : index + 3])
        rgb_rows.append(bytes(rgb))
    write_rgb_png(dest, width, height, rgb_rows)


def read_png_rows(path: Path) -> tuple[int, int, int, list[bytes]]:
    data = path.read_bytes()
    if not data.startswith(PNG_SIGNATURE):
        raise RuntimeError(f"{path} is not a PNG")

    offset = len(PNG_SIGNATURE)
    ihdr = None
    idat = bytearray()
    while offset < len(data):
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        chunk_type = data[offset + 4 : offset + 8]
        chunk_data = data[offset + 8 : offset + 8 + length]
        offset += 12 + length
        if chunk_type == b"IHDR":
            ihdr = chunk_data
        elif chunk_type == b"IDAT":
            idat.extend(chunk_data)
        elif chunk_type == b"IEND":
            break

    if ihdr is None:
        raise RuntimeError(f"{path} is missing IHDR")

    width, height, bit_depth, color_type, compression, filter_method, interlace = struct.unpack(
        ">IIBBBBB", ihdr
    )
    if bit_depth != 8 or compression != 0 or filter_method != 0 or interlace != 0:
        raise RuntimeError("unsupported PNG encoding; expected 8-bit non-interlaced PNG")
    if color_type not in (2, 6):
        raise RuntimeError(f"unsupported PNG color type {color_type}; expected RGB or RGBA")

    bytes_per_pixel = 4 if color_type == 6 else 3
    stride = width * bytes_per_pixel
    raw = zlib.decompress(bytes(idat))
    rows: list[bytes] = []
    previous = bytes(stride)
    cursor = 0

    for _ in range(height):
        filter_type = raw[cursor]
        cursor += 1
        filtered = raw[cursor : cursor + stride]
        cursor += stride
        row = unfilter_row(filter_type, filtered, previous, bytes_per_pixel)
        rows.append(row)
        previous = row

    return width, height, color_type, rows


def unfilter_row(filter_type: int, row: bytes, previous: bytes, bpp: int) -> bytes:
    out = bytearray(row)
    for index, value in enumerate(row):
        left = out[index - bpp] if index >= bpp else 0
        up = previous[index]
        up_left = previous[index - bpp] if index >= bpp else 0
        if filter_type == 0:
            predictor = 0
        elif filter_type == 1:
            predictor = left
        elif filter_type == 2:
            predictor = up
        elif filter_type == 3:
            predictor = (left + up) // 2
        elif filter_type == 4:
            predictor = paeth(left, up, up_left)
        else:
            raise RuntimeError(f"unsupported PNG filter {filter_type}")
        out[index] = (value + predictor) & 0xFF
    return bytes(out)


def paeth(left: int, up: int, up_left: int) -> int:
    estimate = left + up - up_left
    left_distance = abs(estimate - left)
    up_distance = abs(estimate - up)
    up_left_distance = abs(estimate - up_left)
    if left_distance <= up_distance and left_distance <= up_left_distance:
        return left
    if up_distance <= up_left_distance:
        return up
    return up_left


def write_rgb_png(path: Path, width: int, height: int, rows: list[bytes]) -> None:
    def chunk(name: bytes, payload: bytes) -> bytes:
        crc = binascii.crc32(name + payload) & 0xFFFFFFFF
        return struct.pack(">I", len(payload)) + name + payload + struct.pack(">I", crc)

    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    raw = b"".join(b"\x00" + row for row in rows)
    path.write_bytes(
        PNG_SIGNATURE
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, level=9))
        + chunk(b"IEND", b"")
    )


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
