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
ANDROID_RES = ROOT / "adapters/android-ime/app/src/main/res"
ANDROID_PLAY_ICON = ROOT / "adapters/android-ime/store-listing/app-icon-512.png"
ANDROID_FEATURE_GRAPHIC = ROOT / "adapters/android-ime/store-listing/feature-graphic-1024x500.png"
ANDROID_MARK_SOURCE = ROOT / "logo/logo_design.icon/Assets/logo.png"
ANDROID_ICON_BACKGROUND = (189, 111, 89)  # #BD6F59
ANDROID_ICON_SIZE = 1024
# Pixel Launcher zooms adaptive foregrounds during presentation. Keep the artwork
# comfortably inside Android's theoretical 66/108 dp safe zone so the keyboard's
# lower bar remains visible under real launcher masks and motion effects.
ANDROID_SAFE_ZONE = 500

# Legacy raster launcher-icon sizes (px) per density bucket (Android pre-26 fallback).
ANDROID_MIPMAP_SIZES = {
    "mdpi": 48,
    "hdpi": 72,
    "xhdpi": 96,
    "xxhdpi": 144,
    "xxxhdpi": 192,
}
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
    source = default_icon_source()
    copy_square_png(source, ANDROID_LOGO_PNG, "Android LogoCard")
    generate_android_launcher_icons(ANDROID_MARK_SOURCE)


def generate_android_launcher_icons(mark_source: Path) -> None:
    """Generate a native adaptive icon from the canonical transparent mark.

    Adaptive icon (API 26+): a foreground drawable on a solid background color pulled
    from the logo, referenced by mipmap-anydpi-v26/*.xml — the launcher masks and
    scales it to every density, so it's the icon on modern devices and the Play
    listing. Legacy raster mipmaps (pre-26 fallback) are downscaled via sips; if sips
    is unavailable (CI/bootstrap), they're skipped with a warning since adaptive
    covers all supported devices.
    """
    if not mark_source.exists():
        raise RuntimeError(f"Android mark source is missing: {mark_source}")

    background_hex = "#%02X%02X%02X" % ANDROID_ICON_BACKGROUND
    write_adaptive_icon_resources(background_hex)
    fg_dest = ANDROID_RES / "drawable-nodpi/ic_launcher_foreground.png"
    fg_dest.parent.mkdir(parents=True, exist_ok=True)
    foreground_rows = android_foreground_rows(mark_source)
    write_rgba_png(fg_dest, ANDROID_ICON_SIZE, ANDROID_ICON_SIZE, foreground_rows)
    print(f"generated {fg_dest.relative_to(ROOT)}")

    flat_rows = composite_rgba_rows(foreground_rows, ANDROID_ICON_BACKGROUND)
    ANDROID_PLAY_ICON.parent.mkdir(parents=True, exist_ok=True)
    write_rgb_png(
        ANDROID_PLAY_ICON,
        512,
        512,
        downsample_rgb_rows_2x(flat_rows, ANDROID_ICON_SIZE),
    )
    print(f"generated {ANDROID_PLAY_ICON.relative_to(ROOT)}")

    write_rgb_png(
        ANDROID_FEATURE_GRAPHIC,
        1024,
        500,
        android_feature_graphic_rows(foreground_rows),
    )
    print(f"generated {ANDROID_FEATURE_GRAPHIC.relative_to(ROOT)}")

    # Legacy raster fallback (pre-26).
    if shutil.which("sips") is None:
        print(
            "warning: sips unavailable — skipping legacy raster launcher icons; "
            "adaptive icon covers API 26+ devices and the Play listing.",
            file=sys.stderr,
        )
        return
    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        flat_source = Path(tmp.name)
    write_rgb_png(flat_source, ANDROID_ICON_SIZE, ANDROID_ICON_SIZE, flat_rows)
    for bucket, size in ANDROID_MIPMAP_SIZES.items():
        for name in ("ic_launcher", "ic_launcher_round"):
            dest = ANDROID_RES / f"mipmap-{bucket}" / f"{name}.webp"
            dest.parent.mkdir(parents=True, exist_ok=True)
            resize_to_webp(flat_source, dest, size)
    flat_source.unlink(missing_ok=True)
    print(f"generated legacy raster launcher icons ({len(ANDROID_MIPMAP_SIZES)} densities)")


def android_foreground_rows(source: Path) -> list[bytes]:
    """Fit and optically centre the letter and keyboard motif independently."""
    width, height, bit_depth, color_type, rows = read_png_rows(source)
    if color_type != 6:
        raise RuntimeError("Android mark must be a transparent RGBA PNG")
    pixels = [rgba_samples(row, bit_depth) for row in rows]

    occupied_rows = [y for y, row in enumerate(pixels) if any(pixel[3] for pixel in row)]
    if not occupied_rows:
        raise RuntimeError("Android mark is empty")
    # The largest transparent horizontal gap separates the Khmer letter from the
    # keyboard/Morse motif. Treating them as two pieces lets both share a true
    # optical centre even when their source canvases differ.
    split_after = max(
        zip(occupied_rows, occupied_rows[1:]), key=lambda pair: pair[1] - pair[0]
    )[0]
    groups = [(occupied_rows[0], split_after), (next(y for y in occupied_rows if y > split_after), occupied_rows[-1])]
    boxes = [alpha_bounds(pixels, y0, y1) for y0, y1 in groups]
    combined_height = boxes[0][3] - boxes[0][1] + 1 + boxes[1][3] - boxes[1][1] + 1
    source_gap = boxes[1][1] - boxes[0][3] - 1
    combined_height += source_gap
    widest = max(box[2] - box[0] + 1 for box in boxes)
    scale = min(ANDROID_SAFE_ZONE / widest, ANDROID_SAFE_ZONE / combined_height)

    scaled = [resize_crop_rgba(pixels, box, scale) for box in boxes]
    gap = round(source_gap * scale)
    total_height = len(scaled[0]) + gap + len(scaled[1])
    top = (ANDROID_ICON_SIZE - total_height) // 2
    canvas = [bytearray(ANDROID_ICON_SIZE * 4) for _ in range(ANDROID_ICON_SIZE)]
    for piece in scaled:
        piece_width = len(piece[0]) // 4
        left = (ANDROID_ICON_SIZE - piece_width) // 2
        for row in piece:
            canvas[top][left * 4 : left * 4 + len(row)] = row
            top += 1
        if piece is scaled[0]:
            top += gap
    return [bytes(row) for row in canvas]


def alpha_bounds(pixels: list[list[tuple[int, int, int, int]]], y0: int, y1: int) -> tuple[int, int, int, int]:
    points = [(x, y) for y in range(y0, y1 + 1) for x, pixel in enumerate(pixels[y]) if pixel[3]]
    return min(x for x, _ in points), y0, max(x for x, _ in points), y1


def resize_crop_rgba(
    pixels: list[list[tuple[int, int, int, int]]], box: tuple[int, int, int, int], scale: float
) -> list[bytes]:
    left, top, right, bottom = box
    source_width, source_height = right - left + 1, bottom - top + 1
    target_width, target_height = max(1, round(source_width * scale)), max(1, round(source_height * scale))
    output = []
    for target_y in range(target_height):
        source_y = min(bottom, top + int(target_y / scale))
        row = bytearray()
        for target_x in range(target_width):
            source_x = min(right, left + int(target_x / scale))
            row.extend(pixels[source_y][source_x])
        output.append(bytes(row))
    return output


def composite_rgba_rows(rows: list[bytes], background: tuple[int, int, int]) -> list[bytes]:
    output = []
    for row in rows:
        rgb = bytearray()
        for red, green, blue, alpha in rgba_samples(row, 8):
            rgb.extend(composite(channel, bg, alpha) for channel, bg in zip((red, green, blue), background))
        output.append(bytes(rgb))
    return output


def downsample_rgb_rows_2x(rows: list[bytes], size: int) -> list[bytes]:
    """Downsample a square RGB image by exactly 2× with box filtering."""
    if size % 2 or len(rows) != size or any(len(row) != size * 3 for row in rows):
        raise RuntimeError("expected an even square RGB image")
    output = []
    for y in range(0, size, 2):
        row = bytearray()
        for x in range(0, size, 2):
            offset = x * 3
            for channel in range(3):
                total = (
                    rows[y][offset + channel]
                    + rows[y][offset + 3 + channel]
                    + rows[y + 1][offset + channel]
                    + rows[y + 1][offset + 3 + channel]
                )
                row.append((total + 2) // 4)
        output.append(bytes(row))
    return output


def android_feature_graphic_rows(foreground_rows: list[bytes]) -> list[bytes]:
    """Create a landscape Play graphic that extends the icon into a keyboard scene."""
    width, height = 1024, 500
    ink = (20, 16, 27)
    terracotta = ANDROID_ICON_BACKGROUND
    amber = (233, 138, 78)
    ivory = (244, 236, 226)
    canvas = []
    for y in range(height):
        mix = y / (height - 1)
        color = tuple(round(ink[channel] * (1 - mix * 0.18) + terracotta[channel] * mix * 0.18) for channel in range(3))
        canvas.append(bytearray(color * width))

    # Soft brand planes keep the edges useful as crop space without becoming busy.
    draw_rounded_rect(canvas, 62, 54, 900, 392, 54, (255, 255, 255), 10)
    draw_rounded_rect(canvas, 92, 78, 352, 344, 44, terracotta, 235)

    pixels = [rgba_samples(row, 8) for row in foreground_rows]
    occupied = [(x, y) for y, row in enumerate(pixels) for x, pixel in enumerate(row) if pixel[3]]
    box = (
        min(x for x, _ in occupied),
        min(y for _, y in occupied),
        max(x for x, _ in occupied),
        max(y for _, y in occupied),
    )
    mark = resize_crop_rgba(pixels, box, 300 / (box[3] - box[1] + 1))
    paste_rgba(canvas, mark, 268 - len(mark[0]) // 8, 100)

    # Abstract candidate strip and key rows: recognisably an IME without tiny text.
    draw_rounded_rect(canvas, 506, 101, 405, 62, 24, ivory, 24)
    for x, w in ((530, 88), (632, 116), (762, 124)):
        draw_rounded_rect(canvas, x, 119, w, 25, 12, ivory, 150)
    key_rows = [
        (506, 185, 5, 70),
        (526, 263, 4, 79),
        (566, 341, 3, 86),
    ]
    for row_index, (start_x, y, count, key_width) in enumerate(key_rows):
        for index in range(count):
            color = amber if row_index == 1 and index == 2 else ivory
            alpha = 235 if color == amber else 42
            draw_rounded_rect(canvas, start_x + index * (key_width + 12), y, key_width, 60, 15, color, alpha)
    return [bytes(row) for row in canvas]


def paste_rgba(canvas: list[bytearray], image: list[bytes], left: int, top: int) -> None:
    for y, source_row in enumerate(image):
        if not 0 <= top + y < len(canvas):
            continue
        for x, (red, green, blue, alpha) in enumerate(rgba_samples(source_row, 8)):
            if alpha == 0 or not 0 <= left + x < len(canvas[0]) // 3:
                continue
            offset = (left + x) * 3
            destination = canvas[top + y]
            for channel, foreground in enumerate((red, green, blue)):
                destination[offset + channel] = composite(foreground, destination[offset + channel], alpha)


def draw_rounded_rect(
    rows: list[bytearray], x: int, y: int, width: int, height: int, radius: int,
    color: tuple[int, int, int], alpha: int,
) -> None:
    for py in range(y, y + height):
        for px in range(x, x + width):
            dx = max(x + radius - px, 0, px - (x + width - radius - 1))
            dy = max(y + radius - py, 0, py - (y + height - radius - 1))
            if dx * dx + dy * dy > radius * radius:
                continue
            offset = px * 3
            for channel, foreground in enumerate(color):
                rows[py][offset + channel] = composite(foreground, rows[py][offset + channel], alpha)


def write_adaptive_icon_resources(background_hex: str) -> None:
    anydpi = ANDROID_RES / "mipmap-anydpi-v26"
    anydpi.mkdir(parents=True, exist_ok=True)
    xml = (
        '<?xml version="1.0" encoding="utf-8"?>\n'
        '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
        '    <background android:drawable="@color/ic_launcher_background" />\n'
        '    <foreground android:drawable="@drawable/ic_launcher_foreground" />\n'
        '</adaptive-icon>\n'
    )
    (anydpi / "ic_launcher.xml").write_text(xml)
    (anydpi / "ic_launcher_round.xml").write_text(xml)

    values = ANDROID_RES / "values"
    values.mkdir(parents=True, exist_ok=True)
    (values / "ic_launcher_background.xml").write_text(
        '<?xml version="1.0" encoding="utf-8"?>\n'
        '<resources>\n'
        f'    <color name="ic_launcher_background">{background_hex}</color>\n'
        '</resources>\n'
    )


def resize_to_webp(source: Path, dest: Path, size: int) -> None:
    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        tmp_png = Path(tmp.name)
    try:
        subprocess.run(
            ["sips", "--resampleHeightWidth", str(size), str(size), str(source), "--out", str(tmp_png)],
            check=True, capture_output=True, text=True,
        )
        if shutil.which("cwebp") is not None:
            subprocess.run(
                ["cwebp", "-quiet", "-lossless", str(tmp_png), "-o", str(dest)],
                check=True, capture_output=True, text=True,
            )
        else:
            # No cwebp: Android also accepts PNG mipmaps. Write .png alongside so the
            # build still finds a raster fallback.
            shutil.copyfile(tmp_png, dest.with_suffix(".png"))
    finally:
        tmp_png.unlink(missing_ok=True)


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
    if shutil.which("sips") is None:
        width, height, _bit_depth, color_type, _rows = read_png_rows(path)
        values = {
            "pixelWidth": str(width),
            "pixelHeight": str(height),
            "hasAlpha": "yes" if color_type == 6 else "no",
        }
        return "\n".join(f"{key}: {values[key]}" for key in keys if key in values)

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
    write_png(path, width, height, 2, rows)


def write_rgba_png(path: Path, width: int, height: int, rows: list[bytes]) -> None:
    write_png(path, width, height, 6, rows)


def write_png(path: Path, width: int, height: int, color_type: int, rows: list[bytes]) -> None:
    def chunk(name: bytes, payload: bytes) -> bytes:
        crc = binascii.crc32(name + payload) & 0xFFFFFFFF
        return struct.pack(">I", len(payload)) + name + payload + struct.pack(">I", crc)

    ihdr = struct.pack(">IIBBBBB", width, height, 8, color_type, 0, 0, 0)
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
