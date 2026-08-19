#!/usr/bin/env python3
"""Type into a real GTK app through a live IBus daemon and report what the engine did.

The layer no other test reaches: the installed Python adapter, the IBus panel, and the
candidate popup as a user actually sees them. Bridge tests drive the stdio protocol directly
and so cannot catch an adapter that never renders, an engine that is not selected, or a
composition the IME never receives.

Deliberately assertion-light about *content* — the Lexicon and any span-proposal provider
differ per install. It asserts the pipeline is alive (keystrokes reached the engine and
produced candidates) and prints what was rendered so a human can judge the rest.

    python3 scripts/platforms/linux/ibus/gedit_smoke.py --text tonle --screenshot /tmp/shot.png
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import time
from pathlib import Path

LOG_PATH = Path("~/.cache/khmerime/ibus_engine.log").expanduser()


def _fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def require_x_display() -> str:
    display = os.environ.get("DISPLAY")
    if not display:
        _fail("no DISPLAY; this smoke test needs a running X session")
    return display


def select_engine(engine: str) -> None:
    """IBus must actually be on our engine, or keystrokes arrive as plain Latin text.

    This is a real failure mode, not a hypothetical: a first run of this check typed straight
    into the document because the active engine was still `xkb:us::eng`.
    """
    subprocess.run(["ibus", "engine", engine], check=False, capture_output=True)
    time.sleep(1.0)
    active = subprocess.run(["ibus", "engine"], capture_output=True, text=True).stdout.strip()
    if active != engine:
        _fail(f"could not select the {engine!r} engine (active: {active!r})")
    print(f"engine: {active}")


def find_window(display, wm_class_fragment: str):
    from Xlib import X  # noqa: F401  (imported for callers' side effects)

    def walk(win):
        found = []
        try:
            for child in win.query_tree().children:
                try:
                    cls = child.get_wm_class()
                    if cls and wm_class_fragment in str(cls).lower():
                        geometry = child.get_geometry()
                        if geometry.width > 300 and geometry.height > 200:
                            found.append(child)
                except Exception:
                    pass
                found += walk(child)
        except Exception:
            pass
        return found

    return walk(display.screen().root)


def type_text(display, window, text: str, per_key_delay: float) -> None:
    from Xlib import X
    from Xlib.ext import xtest
    from Xlib.XK import string_to_keysym

    window.set_input_focus(X.RevertToParent, X.CurrentTime)
    window.configure(stack_mode=X.Above)
    display.sync()
    time.sleep(1.0)

    for char in text:
        keycode = display.keysym_to_keycode(string_to_keysym(char))
        xtest.fake_input(display, X.KeyPress, keycode)
        display.sync()
        time.sleep(per_key_delay)
        xtest.fake_input(display, X.KeyRelease, keycode)
        display.sync()
        time.sleep(per_key_delay)


def engine_log_tail(since_bytes: int) -> list[str]:
    if not LOG_PATH.exists():
        return []
    with LOG_PATH.open("r", encoding="utf-8", errors="replace") as handle:
        handle.seek(since_bytes)
        return handle.read().splitlines()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--text", default="tonle", help="roman text to type")
    parser.add_argument("--engine", default="khmerime")
    parser.add_argument("--settle", type=float, default=3.0, help="seconds to wait for the debounced refine")
    parser.add_argument("--per-key-delay", type=float, default=0.12)
    parser.add_argument("--screenshot", default="")
    args = parser.parse_args()

    require_x_display()
    try:
        from Xlib import display as xdisplay
    except ImportError:
        _fail("python3-xlib is required (apt install python3-xlib)")

    select_engine(args.engine)
    log_offset = LOG_PATH.stat().st_size if LOG_PATH.exists() else 0

    editor = subprocess.Popen(["gedit"])
    try:
        time.sleep(6.0)
        display = xdisplay.Display()
        windows = find_window(display, "gedit")
        if not windows:
            _fail("no gedit window appeared")

        print(f"typing {args.text!r}...")
        type_text(display, windows[0], args.text, args.per_key_delay)
        time.sleep(args.settle)

        if args.screenshot:
            subprocess.run(["import", "-window", "root", args.screenshot], check=False)
            print(f"screenshot: {args.screenshot}")

        lines = engine_log_tail(log_offset)
        key_events = [line for line in lines if "key_event" in line]
        if not key_events:
            _fail(
                "the engine received no key events — IBus is not routing this window "
                "through the KhmerIME engine"
            )

        last = key_events[-1]
        preedit = re.search(r"preedit='([^']*)'", last)
        candidates = re.search(r"cand=(\d+)", last)
        print(f"key events: {len(key_events)}")
        print(f"final preedit: {preedit.group(1) if preedit else '?'}")
        print(f"candidates: {candidates.group(1) if candidates else '?'}")

        if preedit and preedit.group(1) != args.text:
            _fail(f"preedit {preedit.group(1)!r} does not match typed {args.text!r}")
        if not candidates or candidates.group(1) == "0":
            _fail("the composition produced no candidates")

        refines = [line for line in lines if "refine_composition" in line]
        print(f"refines observed: {len(refines)}")
        for line in refines[-3:]:
            print(f"  {line.split(' ', 1)[-1]}")

        print("OK")
        return 0
    finally:
        editor.terminate()
        try:
            editor.wait(timeout=5)
        except subprocess.TimeoutExpired:
            editor.kill()


if __name__ == "__main__":
    raise SystemExit(main())
