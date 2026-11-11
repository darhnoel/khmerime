"""KhmerIME system architecture, one still, drawn with Codimate.

Rules: codimate-templates/.agents/skills/codimate-diagram-design (8 nodes, orthogonal
connectors, labels on masks, two accents: the session and the core).

Run:  <codimate venv>/bin/python system_architecture.py [--light]  ->  system_architecture[-light].png
Sources: Cargo.toml workspace members, CONTEXT.md "Relationships",
docs/provider-architecture.md.
"""
from pathlib import Path
import math
import os
import sys

import codimate as cm

W, H = 1920, 1080
FONTS = Path(os.environ.get("CODIMATE_FONTS", Path.home() / "Developments/codimate-templates/studio/fonts"))
SANS = str(FONTS / "Inter-Medium.otf")
BOLD = str(FONTS / "Inter-Bold.otf")
MONO = str(FONTS / "IBMPlexMono-Regular.ttf")

LIGHT = "--light" in sys.argv  # writes system_architecture-light.png
if LIGHT:
    PAPER, CARD, EDGE = "#f7f7f5", "#ffffff", "#d4d4da"
    INK, DIM, LINK = "#16161a", "#5c5c66", "#9a9aa4"
    ACCENT, BLUE = "#d2600f", "#2f6fd6"
else:
    PAPER, CARD, EDGE = "#0b0b0d", "#17171b", "#2c2c32"
    INK, DIM, LINK = "#f2f2f4", "#a0a0aa", "#5a5a64"
    ACCENT, BLUE = "#ff8a3d", "#6aa8ff"

CW, CH = 360, 132  # card size

# name: (centre x, centre y, title, what it does, path, style)
NODES = {
    "desk": (276, 300, "Desktop IMEs", "IBus · TSF · IMK", "linux-ibus · windows-tsf · macos-imk", "wide"),
    "mob": (276, 540, "Mobile keyboards", "iOS · Android", "ios-keyboard · android-ime", "wide"),
    "app": (276, 780, "Dioxus app", "web and desktop", "apps/dioxus-app via roman_lookup", "wide"),
    "sess": (780, 540, "ImeSession", "composition, preedit, commit", "crates/session", "accent"),
    "core": (1240, 540, "Transliterator", "decode, rank, segment", "crates/core", "accent"),
    "prov": (1240, 228, "Model provider", "optional span proposals", "SpanProposalProvider", "dashed"),
    "data": (1676, 420, "Dictionary image", "lexicon built from data/", "SharedTransliteratorData", "plain"),
    "conf": (1676, 660, "Config Store", "lexicon packs, suggestions", "crates/khmerime_config", "plain"),
}


def text(sc, key, words, x, y, size, color=INK, font=SANS, anchor="c"):
    w, _ = cm.measure(words, size, font)
    cx = x + w / 2 if anchor == "l" else x - w / 2 if anchor == "r" else x
    sc.text(key, words, size=size, font=font, at=(cx, y)).fill(color).on(layer=8)


def dashed(sc, key, a, b, color, w=1.5, dash=10, gap=7):
    n = max(1, int(math.dist(a, b) // (dash + gap)))
    for i in range(n + 1):
        t0 = i * (dash + gap) / math.dist(a, b)
        t1 = min(1.0, (i * (dash + gap) + dash) / math.dist(a, b))
        if t0 >= 1:
            break
        p = (a[0] + (b[0] - a[0]) * t0, a[1] + (b[1] - a[1]) * t0)
        q = (a[0] + (b[0] - a[0]) * t1, a[1] + (b[1] - a[1]) * t1)
        sc.line((key, i), start=p, end=q, w=w).fill(color).on(layer=2)


def card(sc, key, x, y, title, role, path, style):
    edge = {"accent": ACCENT, "dashed": None}.get(style, EDGE)
    cw = 400 if style == "wide" else CW
    r = sc.rect((key, "box"), w=cw, h=CH, at=(x, y)).round(8).on(layer=1)
    r.fill(CARD, edge=edge, edge_w=1.5) if edge else r.fill(CARD)
    if style == "dashed":
        l, t, rr, b = x - CW / 2, y - CH / 2, x + CW / 2, y + CH / 2
        for i, (p, q) in enumerate([((l, t), (rr, t)), ((rr, t), (rr, b)), ((rr, b), (l, b)), ((l, b), (l, t))]):
            dashed(sc, (key, "edge", i), p, q, BLUE)
    left = x - cw / 2 + 24
    text(sc, (key, "t"), title, left, y - 34, 30, ACCENT if style == "accent" else INK, BOLD, "l")
    text(sc, (key, "r"), role, left, y + 4, 22, DIM, SANS, "l")
    text(sc, (key, "p"), path, left, y + 38, 15 if style == "wide" else 17, BLUE if style == "dashed" else DIM, MONO, "l")


def route(sc, key, pts, color=LINK, w=2.0, dash=False, radius=8):
    """Orthogonal polyline with rounded corners and an arrowhead at the end."""
    path = [pts[0]]
    for a, b, c in zip(pts, pts[1:], pts[2:]):
        ux, uy = (b[0] - a[0], b[1] - a[1])
        vx, vy = (c[0] - b[0], c[1] - b[1])
        lu, lv = math.hypot(ux, uy), math.hypot(vx, vy)
        p = (b[0] - ux / lu * radius, b[1] - uy / lu * radius)
        q = (b[0] + vx / lv * radius, b[1] + vy / lv * radius)
        path += [p, *[(p[0] * (1 - s) ** 2 + 2 * b[0] * s * (1 - s) + q[0] * s * s,
                       p[1] * (1 - s) ** 2 + 2 * b[1] * s * (1 - s) + q[1] * s * s) for s in (0.25, 0.5, 0.75)], q]
    end, before = pts[-1], pts[-2]
    ux, uy = end[0] - before[0], end[1] - before[1]
    lu = math.hypot(ux, uy)
    tip_start = (end[0] - ux / lu * 18, end[1] - uy / lu * 18)
    path.append(tip_start)
    for i, (a, b) in enumerate(zip(path, path[1:])):
        if dash:
            dashed(sc, (key, i), a, b, color, w)
        else:
            sc.line((key, i), start=a, end=b, w=w).fill(color).on(layer=2)
    sc.arrow((key, "head"), start=tip_start, end=end, w=w, head=14).fill(color).on(layer=2)


def tag(sc, key, words, x, y, color=DIM):
    w, h = cm.measure(words, 16, MONO)
    sc.rect((key, "mask"), w=w + 16, h=h + 8, at=(x, y)).fill(PAPER).round(4).on(layer=6)
    text(sc, (key, "w"), words, x, y, 16, color, MONO)


def view(_):
    sc = cm.Scene()
    sc.rect("ground", w=W, h=H, at=(W / 2, H / 2)).fill(PAPER).on(layer=-5)

    text(sc, "title", "KhmerIME system architecture", 76, 80, 40, INK, BOLD, "l")
    text(sc, "sub", "One shared Rust engine. Each platform is a thin adapter around it.", 76, 128, 22, DIM, SANS, "l")

    # Platforms -> ImeSession: three routes fanned onto the session's left edge.
    sx = NODES["sess"][0] - CW / 2
    px = NODES["desk"][0] + 200
    mid = (px + sx) / 2
    for k, y_in in (("desk", 490), ("mob", 540)):
        y0 = NODES[k][1]
        pts = [(px, y0), (sx, y0)] if y0 == y_in else [(px, y0), (mid, y0), (mid, y_in), (sx, y_in)]
        route(sc, ("r", k), pts)
    tag(sc, "keys", "KEY EVENTS", mid, 516)
    # The Dioxus app skips ImeSession and calls the Transliterator directly.
    route(sc, ("r", "app"), [(px, 780), (1180, 780), (1180, 540 + CH / 2)])
    tag(sc, "direct", "DIRECT CALLS", 780, 780)

    # ImeSession -> Transliterator
    cx_l = NODES["core"][0] - CW / 2
    route(sc, "r_sc", [(sx + CW, 540), (cx_l, 540)], ACCENT, 2.5)
    tag(sc, "decode", "DECODE", (sx + CW + cx_l) / 2, 512, ACCENT)

    # Model provider -> Transliterator (dashed, optional, off by default)
    route(sc, "r_pv", [(1240, 228 + CH / 2), (1240, 540 - CH / 2)], BLUE, 2.0, dash=True)
    tag(sc, "prop", "SPAN PROPOSALS", 1240, 384, BLUE)

    # Transliterator -> Dictionary image, Config Store (fanned on the core's right edge)
    cr = NODES["core"][0] + CW / 2
    dl = NODES["data"][0] - CW / 2
    jx = (cr + dl) / 2
    route(sc, "r_dt", [(cr, 512), (jx, 512), (jx, 420), (dl, 420)])
    route(sc, "r_cf", [(cr, 568), (jx, 568), (jx, 660), (dl, 660)])

    for k, (x, y, *rest) in NODES.items():
        card(sc, k, x, y, *rest)

    # Legend strip
    ly = 990
    sc.line("lg_rule", start=(76, ly - 40), end=(W - 76, ly - 40), w=1).fill(EDGE)
    sc.line("lg_a", start=(76, ly), end=(124, ly), w=2.5).fill(ACCENT)
    text(sc, "lg_at", "the typing path of the native keyboards", 140, ly, 20, DIM, SANS, "l")
    dashed(sc, "lg_b", (620, ly), (668, ly), BLUE, 2)
    text(sc, "lg_bt", "optional, off by default (cargo feature model-provider)", 684, ly, 20, DIM, SANS, "l")
    text(sc, "lg_c", "Results flow back the same way: the session returns a snapshot, and the adapter draws it.",
         W - 76, ly + 40, 18, DIM, SANS, "r")
    return sc


if __name__ == "__main__":
    cm.canvas(W, H)
    out = Path(__file__).with_name("system_architecture-light.png" if LIGHT else "system_architecture.png")
    exp = cm.explain(trace=cm.trace(lambda st, emit: None, {}), view=view)
    exp.frame_at(0.1, str(out), scale=1)
    print(exp.covered())
