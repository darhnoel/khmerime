"""Focused document theme tests for the Online Beta (ADR-0023).

These run against a fast static harness (tests/theme/harness.html) that links the
real assets/main.css — no WASM build — and assert *computed* styles via Playwright.
ADR-0023 keeps ADR-0010's load-bearing rule: candidate and preedit text stay on
opaque surfaces meeting WCAG AA contrast in both light and dark themes.
"""

import re
import socket
import subprocess
import sys
import time
from pathlib import Path
from urllib.request import urlopen

import pytest
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
HOST = "127.0.0.1"
HARNESS = "/tests/theme/harness.html"


def _free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind((HOST, 0))
        return sock.getsockname()[1]


def _wait(url: str, timeout_s: float = 15.0) -> None:
    deadline = time.time() + timeout_s
    last = None
    while time.time() < deadline:
        try:
            with urlopen(url, timeout=1.0) as resp:
                if resp.status == 200:
                    return
        except Exception as exc:  # pragma: no cover - startup probe
            last = exc
        time.sleep(0.1)
    raise RuntimeError(f"harness server not ready: {last!r}")


@pytest.fixture(scope="module")
def server():
    port = _free_port()
    proc = subprocess.Popen(
        [sys.executable, "-m", "http.server", str(port), "--bind", HOST],
        cwd=str(ROOT),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    base = f"http://{HOST}:{port}"
    try:
        _wait(base + HARNESS)
        yield base
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


@pytest.fixture(scope="module")
def browser():
    with sync_playwright() as pw:
        b = pw.chromium.launch()
        try:
            yield b
        finally:
            b.close()


@pytest.fixture()
def page(browser, server):
    ctx = browser.new_context()
    pg = ctx.new_page()
    pg.goto(server + HARNESS, wait_until="networkidle")
    try:
        yield pg
    finally:
        ctx.close()


# --- color / contrast helpers (WCAG 2.x) ------------------------------------
def _rgba(value: str):
    nums = re.findall(r"[\d.]+", value)
    r, g, b = (float(x) for x in nums[:3])
    a = float(nums[3]) if len(nums) > 3 else 1.0
    return (r, g, b, a)


def _luminance(r: float, g: float, b: float) -> float:
    def chan(c: float) -> float:
        c /= 255.0
        return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4

    return 0.2126 * chan(r) + 0.7152 * chan(g) + 0.0722 * chan(b)


def _contrast(c1, c2) -> float:
    l1, l2 = _luminance(*c1[:3]), _luminance(*c2[:3])
    hi, lo = max(l1, l2), min(l1, l2)
    return (hi + 0.05) / (lo + 0.05)


def _hex_or_rgb(value: str):
    value = value.strip()
    if value.startswith("#"):
        h = value[1:]
        if len(h) == 3:
            h = "".join(c * 2 for c in h)
        r, g, b = (int(h[i : i + 2], 16) for i in (0, 2, 4))
        return (float(r), float(g), float(b), 1.0)
    return _rgba(value)


def _computed(page, selector: str, prop: str) -> str:
    return page.eval_on_selector(
        selector, f"el => getComputedStyle(el).{prop}"
    )


def _token(page, name: str) -> str:
    return page.eval_on_selector(
        ":root", f"el => getComputedStyle(el).getPropertyValue('{name}')"
    )


# --- behaviors ---------------------------------------------------------------
def test_system_default_is_opaque(page):
    """System theme always resolves to an opaque workspace ground."""
    r, g, b, a = _rgba(_computed(page, "body", "backgroundColor"))
    assert a == 1.0, f"workspace ground must be opaque, got alpha={a}"


def test_accent_is_teal(page):
    """The focused workspace uses the agreed teal accent."""
    r, g, b, _ = _hex_or_rgb(_token(page, "--accent"))
    assert g > r and b > r, f"accent should be teal (g and b > r), got ({r},{g},{b})"


def test_candidate_text_is_legible_on_solid_surface(page):
    """Candidate text sits on an opaque surface at >= AA 4.5:1."""
    surface = _rgba(_computed(page, ".suggestion-popup", "backgroundColor"))
    word = _rgba(_computed(page, ".suggestion-popup .suggestion-word", "color"))
    assert surface[3] == 1.0, f"candidate surface must be opaque behind text, got alpha={surface[3]}"
    ratio = _contrast(word, surface)
    assert ratio >= 4.5, f"candidate text must meet WCAG AA 4.5:1, got {ratio:.2f}:1"


def test_preedit_focused_text_is_legible(page):
    """The focused composition (preedit) text reads clearly on its solid surface."""
    text = _rgba(_computed(page, ".composition-preview-text", "color"))
    bg = _rgba(_computed(page, ".composition-preview", "backgroundColor"))
    assert bg[3] == 1.0, f"preedit surface must be solid behind text, got alpha={bg[3]}"
    ratio = _contrast(text, bg)
    assert ratio >= 4.5, f"preedit text must meet WCAG AA 4.5:1, got {ratio:.2f}:1"


def test_composition_caret_is_visible(page):
    """The typing caret must stand out against the preedit surface (>=3:1 UI contrast)."""
    caret = _rgba(_computed(page, ".composition-caret", "borderRightColor"))
    bg = _rgba(_computed(page, ".composition-preview", "backgroundColor"))
    ratio = _contrast(caret, bg)
    assert ratio >= 3.0, f"caret must be visible on the preedit surface (>=3:1), got {ratio:.2f}:1"


def test_explicit_dark_theme_keeps_candidate_contrast(page):
    page.eval_on_selector(":root", "el => el.setAttribute('data-theme', 'dark')")
    surface = _rgba(_computed(page, ".suggestion-popup", "backgroundColor"))
    word = _rgba(_computed(page, ".suggestion-popup .suggestion-word", "color"))
    assert surface[3] == 1.0
    assert _contrast(word, surface) >= 4.5


@pytest.mark.parametrize("theme", ["light", "dark"])
@pytest.mark.parametrize("palette", ["angkor", "lotus", "forest"])
def test_coordinated_palettes_keep_text_and_accent_contrast(page, theme, palette):
    page.eval_on_selector(
        ":root",
        "(el, attrs) => { el.dataset.theme = attrs.theme; el.dataset.palette = attrs.palette; }",
        {"theme": theme, "palette": palette},
    )
    surface = _hex_or_rgb(_token(page, "--surface-strong"))
    ink = _hex_or_rgb(_token(page, "--ink"))
    accent = _hex_or_rgb(_token(page, "--accent-strong"))
    assert _contrast(ink, surface) >= 4.5
    assert _contrast(accent, surface) >= 3.0
