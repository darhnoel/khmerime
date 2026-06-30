#!/usr/bin/env python3
"""Generate mobile logo assets from local design exports.

This script is intentionally in scripts/dev because the logo sources are design
exports used to feed platform asset catalogs. Generated PNGs stay ignored.
"""

from __future__ import annotations

import argparse
import binascii
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ICON_DEFAULT_EXPORT = ROOT / "logo/logo_design_exports/logo_design-iOS-Default-1024x1024@1x.png"

IOS_ASSETS = ROOT / "adapters/ios-keyboard/swift/KhmerIME/Assets.xcassets"
IOS_LOGO_PNG = IOS_ASSETS / "LogoCard.imageset/logocard.png"
IOS_APPICON_PNG = IOS_ASSETS / "AppIcon.appiconset/appicon-1024.png"
IOS_ABA_SOURCE = ROOT / "site/download/assets/my-aba-cropped.png"
IOS_ABA_DEST = IOS_ASSETS / "ABAQR.imageset/aba.png"

ANDROID_LOGO_PNG = ROOT / "adapters/android-ime/app/src/main/res/drawable-nodpi/khmerime_logo_card.png"

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "platform",
        choices=("ios", "android", "all"),
        help="Platform assets to generate.",
    )
    args = parser.parse_args(argv)

    if args.platform in ("ios", "all"):
        generate_ios_assets()
    if args.platform in ("android", "all"):
        generate_android_assets()
    return 0


def generate_ios_assets() -> None:
    icon_source = default_icon_source()
    copy_square_png(icon_source, IOS_LOGO_PNG, "LogoCard")
    flatten_png_alpha(icon_source, IOS_APPICON_PNG)
    verify_app_icon(IOS_APPICON_PNG)
    print(f"generated {IOS_APPICON_PNG.relative_to(ROOT)}")

    IOS_ABA_DEST.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(IOS_ABA_SOURCE, IOS_ABA_DEST)
    print(f"generated {IOS_ABA_DEST.relative_to(ROOT)}")


def generate_android_assets() -> None:
    copy_square_png(default_icon_source(), ANDROID_LOGO_PNG, "Android LogoCard")


def default_icon_source() -> Path:
    override = os.environ.get("KHMERIME_LOGO_SOURCE")
    if override:
        override_path = Path(override)
        if override_path.exists():
            return override_path
        print(f"warning: KHMERIME_LOGO_SOURCE does not exist: {override_path}", file=sys.stderr)
    if ICON_DEFAULT_EXPORT.exists():
        return ICON_DEFAULT_EXPORT

    fallback = Path(tempfile.gettempdir()) / "khmerime-mobile-logo-fallback-1024.png"
    write_temporary_logo(fallback)
    print(
        "warning: local logo design export is missing; "
        f"using temporary generated logo at {fallback}",
        file=sys.stderr,
    )
    return fallback


def write_temporary_logo(path: Path) -> None:
    """Write a dependency-free temporary logo for CI/bootstrap builds.

    The real source lives under local design exports and is intentionally not
    required for Xcode Cloud post-clone. This keeps package builds unblocked;
    designers can still override with KHMERIME_LOGO_SOURCE.
    """
    size = 1024
    background = (226, 143, 88)
    dark = (33, 29, 42)
    ivory = (255, 248, 232)
    rows: list[bytes] = []
    for y in range(size):
        row = bytearray()
        for x in range(size):
            pixel = background
            if 200 <= x <= 824 and 200 <= y <= 824:
                pixel = dark
            if 280 <= x <= 744 and 280 <= y <= 744:
                pixel = ivory
            if 360 <= x <= 664 and 360 <= y <= 664:
                pixel = dark
            row.extend(pixel)
        rows.append(bytes(row))
    write_rgb_png(path, size, size, rows)


def copy_square_png(source: Path, dest: Path, label: str) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, dest)
    verify_square(dest, label)
    print(f"generated {dest.relative_to(ROOT)}")


def verify_app_icon(path: Path) -> None:
    output = image_metadata(path, "pixelWidth", "pixelHeight", "hasAlpha")
    if metadata_value(output, "pixelWidth") != 1024 or metadata_value(output, "pixelHeight") != 1024:
        raise RuntimeError(f"AppIcon must be 1024x1024:\n{output}")
    if "hasAlpha: no" not in output:
        raise RuntimeError(f"AppIcon must not contain an alpha channel:\n{output}")


def verify_square(path: Path, label: str) -> None:
    output = image_metadata(path, "pixelWidth", "pixelHeight")
    width = metadata_value(output, "pixelWidth")
    height = metadata_value(output, "pixelHeight")
    if width is None or height is None or width != height:
        raise RuntimeError(f"{label} must be square:\n{output}")


def image_metadata(path: Path, *keys: str) -> str:
    args = ["sips"]
    for key in keys:
        args.extend(["-g", key])
    args.append(str(path))
    return subprocess.run(
        args,
        check=True,
        text=True,
        capture_output=True,
    ).stdout


def metadata_value(output: str, key: str) -> int | None:
    marker = f"{key}: "
    for line in output.splitlines():
        line = line.strip()
        if line.startswith(marker):
            return int(line.removeprefix(marker))
    return None


def flatten_png_alpha(source: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    width, height, bit_depth, color_type, rows = read_png_rows(source)
    if color_type == 2:
        write_rgb_png(dest, width, height, rgb_rows_from_truecolor(rows, bit_depth))
        return
    if color_type != 6:
        raise RuntimeError(f"unsupported PNG color type {color_type}; expected RGB or RGBA")

    background = icon_background(rows, width, height, bit_depth)
    rgb_rows = []
    for row in rows:
        rgb = bytearray()
        for red, green, blue, alpha in rgba_samples(row, bit_depth):
            if alpha == 255:
                rgb.extend((red, green, blue))
            elif alpha == 0:
                rgb.extend(background)
            else:
                rgb.extend(
                    (
                        composite(red, background[0], alpha),
                        composite(green, background[1], alpha),
                        composite(blue, background[2], alpha),
                    )
                )
        rgb_rows.append(bytes(rgb))
    write_rgb_png(dest, width, height, rgb_rows)


def icon_background(rows: list[bytes], width: int, height: int, bit_depth: int) -> tuple[int, int, int]:
    sample_points = [
        (width // 2, max(0, height // 10)),
        (width // 2, max(0, height // 6)),
        (width // 4, height // 2),
        ((width * 3) // 4, height // 2),
    ]
    for x, y in sample_points:
        bytes_per_pixel = 8 if bit_depth == 16 else 4
        offset = x * bytes_per_pixel
        red, green, blue, alpha = rgba_samples(rows[y][offset : offset + bytes_per_pixel], bit_depth)[0]
        if alpha >= 250:
            return (red, green, blue)
    return (226, 143, 88)


def rgba_samples(row: bytes, bit_depth: int) -> list[tuple[int, int, int, int]]:
    if bit_depth == 8:
        return [
            (row[index], row[index + 1], row[index + 2], row[index + 3])
            for index in range(0, len(row), 4)
        ]
    if bit_depth == 16:
        return [
            (
                downsample_16(row[index], row[index + 1]),
                downsample_16(row[index + 2], row[index + 3]),
                downsample_16(row[index + 4], row[index + 5]),
                downsample_16(row[index + 6], row[index + 7]),
            )
            for index in range(0, len(row), 8)
        ]
    raise RuntimeError(f"unsupported PNG bit depth {bit_depth}; expected 8 or 16")


def rgb_rows_from_truecolor(rows: list[bytes], bit_depth: int) -> list[bytes]:
    if bit_depth == 8:
        return rows
    if bit_depth != 16:
        raise RuntimeError(f"unsupported PNG bit depth {bit_depth}; expected 8 or 16")
    rgb_rows = []
    for row in rows:
        rgb = bytearray()
        for index in range(0, len(row), 6):
            rgb.extend(
                (
                    downsample_16(row[index], row[index + 1]),
                    downsample_16(row[index + 2], row[index + 3]),
                    downsample_16(row[index + 4], row[index + 5]),
                )
            )
        rgb_rows.append(bytes(rgb))
    return rgb_rows


def downsample_16(high: int, low: int) -> int:
    return (((high << 8) | low) * 255 + 32767) // 65535


def composite(foreground: int, background: int, alpha: int) -> int:
    return ((foreground * alpha) + (background * (255 - alpha)) + 127) // 255


def read_png_rows(path: Path) -> tuple[int, int, int, int, list[bytes]]:
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
    if bit_depth not in (8, 16) or compression != 0 or filter_method != 0 or interlace != 0:
        raise RuntimeError("unsupported PNG encoding; expected 8/16-bit non-interlaced PNG")
    if color_type not in (2, 6):
        raise RuntimeError(f"unsupported PNG color type {color_type}; expected RGB or RGBA")

    bytes_per_sample = 2 if bit_depth == 16 else 1
    samples_per_pixel = 4 if color_type == 6 else 3
    bytes_per_pixel = samples_per_pixel * bytes_per_sample
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

    return width, height, bit_depth, color_type, rows


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
