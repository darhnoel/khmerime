"""Render the download page's typing demos with Codimate: a phone and a desktop window.

Both scenes replay real engine output, keystroke by keystroke, from typing.json
(lookup_cli `suggest` for the candidates, `segmented` for the segment preview).

Run (needs Codimate, its fonts and ffmpeg):
    <codimate venv>/bin/python scripts/web/demo/render_demo.py
Writes site/download/assets/demo/{phone,desktop}.mp4 and a .png poster for each.
Set CODIMATE_FONTS if the fonts are not in ~/Developments/codimate-templates/studio/fonts.
"""
from dataclasses import dataclass
from pathlib import Path
import json
import os
import subprocess
import tempfile

import codimate as cm

HERE = Path(__file__).resolve().parent
OUT = HERE.parents[2] / "site" / "download" / "assets" / "demo"
DATA = json.loads((HERE / "typing.json").read_text(encoding="utf-8"))
ROWS = DATA["keystrokes"]
COMMIT = DATA["commit"]

FONTS = Path(os.environ.get("CODIMATE_FONTS", Path.home() / "Developments/codimate-templates/studio/fonts"))
LATIN = str(FONTS / "Inter-Medium.otf")
LATIN_BOLD = str(FONTS / "Inter-Bold.otf")
KHMER = str(FONTS / "NotoSansKhmer-Regular.ttf")
MONO = str(FONTS / "IBMPlexMono-Regular.ttf")

PAPER = "#f6f4ef"
ACCENT = "#b5441b"


@dataclass
class Typing:
    step: int = -1          # index into ROWS; -1 = nothing typed yet
    key: str = ""           # key pressed at this moment
    tap: bool = False       # the top candidate is being chosen
    committed: bool = False
    cleared: bool = False


class Recorder(Typing):
    """A Typing that keeps a copy of itself each time play() emits."""

    def __init__(self, sink):
        super().__init__()
        object.__setattr__(self, "_sink", sink)


def play(st, emit):
    if isinstance(st, Recorder):
        sink, base = st._sink, emit
        def emit(event, _st=st):
            sink.append(Typing(_st.step, _st.key, _st.tap, _st.committed, _st.cleared))
            base(event)
    emit("start")
    for i, row in enumerate(ROWS):
        st.step, st.key = i, row["roman"][-1]
        emit("key")
    st.key = ""
    emit("look")
    st.tap = True
    emit("tap")
    st.tap, st.committed = False, True
    emit("commit")
    emit("rest")
    st.cleared = True
    emit("clear")


TIMING = cm.Timing(default=0.15, events={"look": 0.6, "tap": 0.35, "commit": 0.3, "rest": 2.4, "clear": 0.5},
                   opening=0.6, final_hold=0.4)


def current(st):
    return ROWS[st.step] if st.step >= 0 and not st.committed and not st.cleared else None


def label(sc, key, words, x, y, size, color, font, anchor="c", layer=5):
    if not words:
        return
    w, _ = cm.measure(words, size, font)
    cx = x + w / 2 if anchor == "l" else x - w / 2 if anchor == "r" else x
    sc.text(key, words, size=size, font=font, at=(cx, y)).fill(color).on(layer=layer)


# ---------------------------------------------------------------- phone ----

PW, PH = 420, 860
SL, ST, SW, SH = 26, 24, 368, 812        # screen left, top, width, height
KEY_UP, KEY_DOWN, KEY_TXT = "#3b3b3f", "#c9c9cf", "#f0f0f2"


def keyboard_rows():
    """Every row on one grid: the same gap everywhere, wide keys fill the row exactly."""
    gap, kw = 5, 31
    full = 10 * kw + 9 * gap                  # row 1 sets the width
    left = SL + (SW - full) / 2

    def lay(y, keys):                         # keys: (name, width) left to right
        out, x = [], left
        for name, w in keys:
            out.append((name, x + w / 2, y, w))
            x += w + gap
        return out

    rows = lay(612, [(c, kw) for c in "qwertyuiop"])
    row2 = [(c, kw) for c in "asdfghjkl"]
    offset = (full - (9 * kw + 8 * gap)) / 2  # row 2 is centred, as on the real keyboard
    rows += [(n, x + offset, y, w) for n, x, y, w in lay(662, row2)]
    side = (full - 7 * kw - 8 * gap) / 2
    rows += lay(712, [("shift", side)] + [(c, kw) for c in "zxcvbnm"] + [("back", side)])
    wide = 44
    space = full - 3 * wide - kw - 4 * gap
    rows += lay(762, [("En", wide), ("123", wide), ("space", space), (".", kw), ("enter", wide)])
    return rows


KEYS = keyboard_rows()


def phone(frame):
    st = frame.state
    row = current(st)
    sc = cm.Scene()
    sc.rect("bg", w=PW, h=PH, at=(PW / 2, PH / 2)).fill(PAPER).on(layer=-9)
    sc.rect("body", w=392, h=836, at=(PW / 2, PH / 2)).fill("#1c1c1f").round(54).on(layer=-8)
    sc.rect("screen", w=SW, h=SH, at=(SL + SW / 2, ST + SH / 2)).fill("#101012").round(44).on(layer=-7)
    label(sc, "time", "9:41", SL + 34, ST + 26, 14, "#e8e8ea", LATIN_BOLD, "l")
    label(sc, "app", "Notes", SL + 24, ST + 72, 15, "#8e8e93", LATIN, "l")

    # The text field: the roman preedit while typing, the Khmer after commit.
    fx, fy = SL + 24, ST + 124
    if st.committed and not st.cleared:
        label(sc, "field", COMMIT, fx, fy, 26, "#f2f2f4", KHMER, "l")
        end = fx + cm.measure(COMMIT, 26, KHMER)[0]
    elif row:
        label(sc, "field", row["roman"], fx, fy, 24, "#f2f2f4", LATIN, "l")
        end = fx + cm.measure(row["roman"], 24, LATIN)[0]
    else:
        end = fx
    if not st.cleared:
        sc.rect("caret", w=2, h=30, at=(end + 1, fy)).fill(ACCENT).on(layer=6)

    # Segment preview, above the keyboard.
    if row and row["segments"]:
        label(sc, "seg_roman", " · ".join(s[0] for s in row["segments"]), SL + SW / 2, 478, 12, "#9a9aa0", LATIN)
        label(sc, "seg_khmer", "  ".join(s[1] for s in row["segments"]), SL + SW / 2, 502, 19, "#f2f2f4", KHMER)

    kbd_bottom = ST + SH - 44
    sc.rect("kbd", w=SW, h=kbd_bottom - 520, at=(SL + SW / 2, (520 + kbd_bottom) / 2)).fill("#1b1b1e").on(layer=-6)
    sc.rect("kbd_round", w=SW, h=88, at=(SL + SW / 2, ST + SH - 44)).fill("#1b1b1e").round(44).on(layer=-6)

    # Candidate chips.
    if row:
        x = SL + 10
        for i, cand in enumerate(row["candidates"]):
            w = cm.measure(cand, 16, KHMER)[0] + 26
            if x + w > SL + SW - 10:
                break
            fill = ACCENT if (st.tap and i == 0) else ("#3a3a40" if i == 0 else "#2a2a2e")
            sc.rect(("chip", i, cand), w=w, h=34, at=(x + w / 2, 552)).fill(fill).round(17).on(layer=2)
            label(sc, ("chip_t", i, cand), cand, x + w / 2, 552, 16, "#f2f2f4", KHMER, layer=3)
            x += w + 8

    for name, x, y, w in KEYS:
        down = st.key == name
        sc.rect(("key", name), w=w, h=42, at=(x, y)).fill(KEY_DOWN if down else KEY_UP).round(6).on(layer=1)
        text = {"shift": "⇧", "back": "⌫", "enter": "↵", "space": "space"}.get(name, name.upper() if len(name) == 1 else name)
        font = LATIN if len(text) > 1 or text.isalpha() or text == "." else None
        size = 13 if len(text) > 1 else (17 if font else 22)
        label(sc, ("key_t", name), text, x, y, size, "#111114" if down else KEY_TXT, font)

    sc.rect("home", w=120, h=4, at=(SL + SW / 2, ST + SH - 12)).fill("#8e8e93").round(2).on(layer=3)
    return sc


# -------------------------------------------------------------- desktop ----

DW, DH = 880, 800                    # near-square, so it fills the page's demo frame
WX, WY, WW, WH = 8, 8, 864, 784
K = 1.3                              # content scale: the frame shows this at ~0.65x


def desktop(frame):
    st = frame.state
    row = current(st)
    sc = cm.Scene()
    sc.rect("bg", w=DW, h=DH, at=(DW / 2, DH / 2)).fill(PAPER).on(layer=-9)
    sc.rect("win", w=WW, h=WH, at=(WX + WW / 2, WY + WH / 2)).fill("#ffffff", edge="#d9d4ca", edge_w=2).round(16).on(layer=-8)
    bar = 52
    sc.rect("bar", w=WW - 4, h=bar, at=(WX + WW / 2, WY + bar / 2 + 1)).fill("#f1eee8").round(15).on(layer=-7)
    sc.rect("bar_sq", w=WW - 4, h=18, at=(WX + WW / 2, WY + bar - 8)).fill("#f1eee8").on(layer=-7)
    sc.line("bar_rule", start=(WX + 1, WY + bar), end=(WX + WW - 1, WY + bar), w=1.5).fill("#e3ded4").on(layer=-6)
    label(sc, "title", "Untitled — Notes", WX + WW / 2, WY + bar / 2, 16, "#6b665f", LATIN)

    tx, ty = WX + 64, WY + bar + 84
    if st.committed and not st.cleared:
        label(sc, "doc", COMMIT, tx, ty, 30 * K, "#1b1a17", KHMER, "l")
        end = tx + cm.measure(COMMIT, 30 * K, KHMER)[0]
    elif row:
        label(sc, "doc", row["roman"], tx, ty, 26 * K, "#1b1a17", LATIN, "l")
        end = tx + cm.measure(row["roman"], 26 * K, LATIN)[0]
        sc.rect("underline", w=end - tx, h=3, at=((tx + end) / 2, ty + 20 * K)).fill(ACCENT).on(layer=4)
    else:
        end = tx
    if not st.cleared:
        sc.rect("caret", w=2.5, h=32 * K, at=(end + 2, ty)).fill("#1b1a17").on(layer=6)

    if row:
        px, py, pw = tx, ty + 44 * K, 380 * K
        has_segments = bool(row["segments"])
        head = 64 * K if has_segments else 0
        rh = 44 * K
        ph = head + len(row["candidates"]) * rh + 12 * K
        sc.rect("pop", w=pw, h=ph, at=(px + pw / 2, py + ph / 2)).fill("#fffdf8", edge="#d9d4ca", edge_w=1.5).round(10).on(layer=7)
        if has_segments:
            label(sc, "p_roman", " · ".join(s[0] for s in row["segments"]), px + 18 * K, py + 20 * K, 13 * K, "#8a857d", MONO, "l", layer=9)
            label(sc, "p_khmer", "  ".join(s[1] for s in row["segments"]), px + 18 * K, py + 46 * K, 18 * K, "#1b1a17", KHMER, "l", layer=9)
            sc.line("p_rule", start=(px + 12 * K, py + head), end=(px + pw - 12 * K, py + head), w=1.5).fill("#e3ded4").on(layer=8)
        for i, cand in enumerate(row["candidates"]):
            cy = py + head + 6 * K + rh / 2 + i * rh
            chosen = st.tap and i == 0
            if i == 0:
                sc.rect(("p_sel", i), w=pw - 12 * K, h=rh - 4 * K, at=(px + pw / 2, cy)).fill(ACCENT if chosen else "#f6e3d9").round(8).on(layer=8)
            label(sc, ("p_n", i), str(i + 1), px + 22 * K, cy, 14 * K, "#ffffff" if chosen else "#8a857d", LATIN, layer=9)
            label(sc, ("p_c", i, cand), cand, px + 44 * K, cy, 20 * K, "#ffffff" if chosen else "#1b1a17", KHMER, "l", layer=9)
    return sc


def render(name, view, size, scale):
    """Draw each state once and hold it, like a real keyboard: no in-between frames.

    Codimate tweens every shape between consecutive states, which makes text
    crossfade and the caret slide. A keyboard updates instantly, so each event is
    drawn as its settled still and ffmpeg holds it for the event's duration.
    """
    cm.canvas(*size)
    exp = cm.explain(trace=cm.trace(play, Typing()), view=view, timing=TIMING)
    OUT.mkdir(parents=True, exist_ok=True)
    # Snapshot every state play() emits, then draw each one as its own one-state
    # scene, so no frame can catch Codimate mid-tween.
    states = []
    play(Recorder(states), lambda event: None)
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        lines = []
        timeline = exp.timeline()
        for i, (start, length, event) in enumerate(timeline):
            snap = states[min(max(i - 1, 0), len(states) - 1)]
            still = tmp / f"{i:03d}.png"
            cm.explain(trace=cm.trace(lambda st, emit: None, snap), view=view).frame_at(0.1, str(still), scale=scale)
            lines += [f"file '{still}'", f"duration {length:.3f}"]
            if event == "rest":
                (OUT / f"{name}.png").write_bytes(still.read_bytes())
        lines.append(lines[-2])  # the concat demuxer needs the last file listed twice
        (tmp / "list.txt").write_text("\n".join(lines) + "\n")
        # One H.264 keyframe per state, each shown for its own duration (variable frame
        # rate): about 27 frames per loop, and no shimmer, because no frame is predicted
        # from another (predicted frames keep re-sharpening text after each change).
        subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-f", "concat", "-safe", "0", "-i", str(tmp / "list.txt"),
                        "-fps_mode", "vfr", "-vf", "format=yuv420p", "-an", "-c:v", "libx264", "-preset", "slow",
                        "-qp", "22", "-g", "1", "-bf", "0", "-movflags", "+faststart",
                        "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709",
                        str(OUT / f"{name}.mp4")], check=True)
    print(f"wrote {OUT / f'{name}.mp4'}")


if __name__ == "__main__":
    import sys
    which = sys.argv[1:] or ["phone", "desktop"]
    if "phone" in which:
        render("phone", phone, (PW, PH), 1.3)
    if "desktop" in which:
        render("desktop", desktop, (DW, DH), 1.5)
