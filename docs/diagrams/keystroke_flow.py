"""How a keystroke flows, as a sequence still, drawn with Codimate.

Same look and rules as system_architecture.py (codimate-diagram-design: 6 lifelines,
10 messages, labels on masks).

Run:  <codimate venv>/bin/python keystroke_flow.py [--light]  ->  keystroke_flow[-light].png
Sources: crates/session/src/ime_session.rs (process_key_event,
refine_segmented_with_visible_refiner), adapters/android-ime/src/lib.rs (refine_with_model,
debounced), CONTEXT.md (Visible Refiner / Commit Refiner, Smart mode).
"""
from pathlib import Path

import codimate as cm

from system_architecture import (
    ACCENT, BLUE, BOLD, CARD, DIM, EDGE, INK, LIGHT, LINK, MONO, PAPER, SANS, H, W,
    dashed, route, tag, text,
)

# name: (title, sublabel, dashed card)
LANES = [
    ("Platform adapter", "native key events", False),
    ("ImeSession", "crates/session", False),
    ("Transliterator", "live, every key", False),
    ("Visible refiner", "75 ms budget", False),
    ("Model provider", "Smart mode only", True),
    ("Commit refiner", "larger budget", False),
]
X0, STEP, CW, CH, TOP = 200, 304, 256, 96, 232
XS = [X0 + STEP * i for i in range(len(LANES))]
BOTTOM = 948


def lane_card(sc, i, title, sub, dash):
    x = XS[i]
    r = sc.rect(("lane", i), w=CW, h=CH, at=(x, TOP)).round(8).on(layer=1)
    if dash:
        r.fill(CARD)
        l, t, rr, b = x - CW / 2, TOP - CH / 2, x + CW / 2, TOP + CH / 2
        for k, (p, q) in enumerate([((l, t), (rr, t)), ((rr, t), (rr, b)), ((rr, b), (l, b)), ((l, b), (l, t))]):
            dashed(sc, ("lane_edge", i, k), p, q, BLUE)
    else:
        r.fill(CARD, edge=EDGE, edge_w=1.5)
    text(sc, ("lt", i), title, x, TOP - 16, 26, INK, BOLD)
    text(sc, ("ls", i), sub, x, TOP + 20, 17, BLUE if dash else DIM, MONO)
    dashed(sc, ("life", i), (x, TOP + CH / 2), (x, BOTTOM), BLUE if dash else EDGE, 1.5, 6, 6)


def msg(sc, key, a, b, y, label, color=LINK, reply=False):
    xa, xb = XS[a], XS[b]
    sign = 1 if xb > xa else -1
    route(sc, key, [(xa + 4 * sign, y), (xb - 4 * sign, y)], color, 2.5 if color == ACCENT else 2.0, dash=reply)
    # Centre the label between the start and the next lifeline, so it never sits on one.
    lx = (xa + XS[a + sign]) / 2
    tag(sc, (key, "tag"), label, lx, y - 20, DIM if color == LINK else color)


def phase(sc, key, y, words):
    sc.line((key, "rule"), start=(72, y + 18), end=(W - 72, y + 18), w=1).fill(EDGE).on(layer=0)
    w, h = cm.measure(words, 18, BOLD)
    sc.rect((key, "mask"), w=w + 20, h=h + 6, at=(72 + w / 2 + 10, y + 18)).fill(PAPER).on(layer=5)
    text(sc, (key, "t"), words, 82, y + 18, 18, DIM, BOLD, "l")


def view(_):
    sc = cm.Scene()
    sc.rect("ground", w=W, h=H, at=(W / 2, H / 2)).fill(PAPER).on(layer=-5)
    text(sc, "title", "How a keystroke flows", 76, 80, 40, INK, BOLD, "l")
    text(sc, "sub", "The live engine answers every key. The model only runs after a pause, off the hot path.",
         76, 128, 22, DIM, SANS, "l")

    for i, lane in enumerate(LANES):
        lane_card(sc, i, *lane)

    phase(sc, "p1", 296, "EVERY KEYSTROKE")
    msg(sc, "m1", 0, 1, 364, "KEY EVENT", ACCENT)
    msg(sc, "m2", 1, 2, 412, "DECODE", ACCENT)
    msg(sc, "m3", 1, 0, 460, "SNAPSHOT", ACCENT, reply=True)

    phase(sc, "p2", 508, "AFTER A PAUSE (DEBOUNCED)")
    msg(sc, "m4", 0, 1, 576, "REFINE")
    msg(sc, "m5", 1, 3, 624, "RE-DECODE")
    msg(sc, "m6", 3, 4, 672, "SPANS", BLUE)
    msg(sc, "m7", 1, 0, 720, "PREVIEW", reply=True)

    phase(sc, "p3", 768, "ON ENTER")
    msg(sc, "m8", 0, 1, 836, "ENTER")
    msg(sc, "m9", 1, 5, 884, "FINAL DECODE")
    msg(sc, "m10", 1, 0, 932, "COMMIT TEXT", reply=True)

    ly = 1006
    sc.line("lg_rule", start=(76, ly - 34), end=(W - 76, ly - 34), w=1).fill(EDGE)
    sc.line("lg_a", start=(76, ly), end=(124, ly), w=2.5).fill(ACCENT)
    text(sc, "lg_at", "the hot path", 140, ly, 20, DIM, SANS, "l")
    dashed(sc, "lg_r", (330, ly), (378, ly), LINK, 2)
    text(sc, "lg_rt", "reply to the adapter", 394, ly, 20, DIM, SANS, "l")
    dashed(sc, "lg_b", (680, ly), (728, ly), BLUE, 2)
    text(sc, "lg_bt", "only with a registered provider; inert in the open-source build", 744, ly, 20, DIM, SANS, "l")
    return sc


if __name__ == "__main__":
    cm.canvas(W, H)
    out = Path(__file__).with_name("keystroke_flow-light.png" if LIGHT else "keystroke_flow.png")
    exp = cm.explain(trace=cm.trace(lambda st, emit: None, {}), view=view)
    exp.frame_at(0.1, str(out), scale=1)
    print(exp.covered())
