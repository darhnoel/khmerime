import os
import re
import socket
import subprocess
import time
from pathlib import Path
from urllib.request import urlopen

import pytest
from playwright.sync_api import expect, sync_playwright
from playwright.sync_api import TimeoutError as PlaywrightTimeoutError


ROOT = Path(__file__).resolve().parents[1]
HOST = "127.0.0.1"


def _free_port(host: str) -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind((host, 0))
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return sock.getsockname()[1]


def _port_open(host: str, port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(0.25)
        return sock.connect_ex((host, port)) == 0


def _wait_for_server(url: str, timeout_s: float = 90.0) -> None:
    deadline = time.time() + timeout_s
    last_error = None
    while time.time() < deadline:
        try:
            with urlopen(url, timeout=2.0) as response:
                if response.status == 200:
                    return
        except Exception as exc:  # pragma: no cover - debug helper
            last_error = exc
        time.sleep(0.5)
    raise RuntimeError(f"web server did not become ready: {last_error!r}")


def _goto_app(page, url: str) -> None:
    for _ in range(2):
        try:
            page.goto(url, wait_until="domcontentloaded", timeout=60_000)
        except PlaywrightTimeoutError:
            page.goto(url, wait_until="commit", timeout=60_000)
        try:
            page.wait_for_selector("[data-testid='editor-input']", state="attached", timeout=12_000)
            return
        except PlaywrightTimeoutError:
            continue


def _manual_candidate_index(page, text: str) -> int:
    popup = page.locator("[data-testid='suggestion-popup']").last
    expect(popup).to_be_visible(timeout=15_000)
    expect(popup.locator(".suggestion button").first).to_be_visible(timeout=15_000)

    words = popup.locator(".suggestion .suggestion-word")
    count = words.count()
    for index in range(count):
        if words.nth(index).inner_text().strip() == text:
            return index
    raise AssertionError(f"manual candidate not found: {text!r}")


def _click_manual_candidate(page, text: str) -> None:
    popup = page.locator("[data-testid='suggestion-popup']").last
    index = _manual_candidate_index(page, text)
    popup.locator(".suggestion button").nth(index).click()


def _manual_active_candidate_word(page) -> str:
    popup = page.locator("[data-testid='suggestion-popup']").last
    expect(popup).to_be_visible(timeout=15_000)
    return popup.locator(".suggestion.active .suggestion-word").last.inner_text().strip()


def _manual_active_candidate_hint(page) -> str:
    popup = page.locator("[data-testid='suggestion-popup']").last
    expect(popup).to_be_visible(timeout=15_000)
    return popup.locator(".suggestion.active .suggestion-roman-hint").last.inner_text().strip()


def _set_editor_caret(page, caret: int) -> int:
    return page.eval_on_selector(
        "[data-testid='editor-input']",
        """(el, pos) => {
            el.focus();
            if (typeof el.setSelectionRange === "function") {
                el.setSelectionRange(pos, pos);
            }
            return typeof el.selectionStart === "number" ? el.selectionStart : -1;
        }""",
        caret,
    )


def _editor_caret(page) -> int:
    return page.eval_on_selector(
        "[data-testid='editor-input']",
        "el => (typeof el.selectionStart === 'number' ? el.selectionStart : -1)",
    )


def _candidate_bar_bottom_px(page) -> float:
    return float(
        page.eval_on_selector(
            ".candidate-bar",
            "el => parseFloat(window.getComputedStyle(el).bottom || '0')",
        )
    )


def _candidate_bar_position(page) -> str:
    return page.eval_on_selector(".candidate-bar", "el => window.getComputedStyle(el).position")


@pytest.fixture(scope="module")
def web_server():
    port = _free_port(HOST)
    base_url = f"http://{HOST}:{port}"

    env = os.environ.copy()
    env["ADDR"] = HOST
    env["PORT"] = str(port)
    env["DX_FEATURES"] = "wfst-decoder"
    process = subprocess.Popen(
        ["bash", "scripts/serve_web_phone.sh"],
        cwd=ROOT,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        text=True,
    )

    try:
        _wait_for_server(base_url)
        yield base_url
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


@pytest.fixture(scope="module")
def playwright_runtime():
    with sync_playwright() as playwright:
        yield playwright


@pytest.fixture(scope="module")
def browser(playwright_runtime):
    browser = playwright_runtime.chromium.launch()
    try:
        yield browser
    finally:
        browser.close()


@pytest.fixture()
def page(browser):
    context = browser.new_context()
    try:
        yield context.new_page()
    finally:
        context.close()


@pytest.fixture()
def mobile_page(browser):
    context = browser.new_context(viewport={"width": 390, "height": 844})
    try:
        yield context.new_page()
    finally:
        context.close()


@pytest.mark.slow
def test_web_ui_desktop_popup_and_live_edit_toggle(web_server: str, page) -> None:
    console_messages = []
    page_errors = []
    page.on("console", lambda msg: console_messages.append(f"{msg.type}: {msg.text}"))
    page.on("pageerror", lambda exc: page_errors.append(str(exc)))
    _goto_app(page, web_server)

    editor = page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    expect(page.locator("[data-testid='engine-status']")).to_have_count(0, timeout=20_000)
    editor.click()
    editor.type("jea")

    popup = page.locator("[data-testid='suggestion-popup']").last
    for message in console_messages:
        print(f"CONSOLE={message}")
    for error in page_errors:
        print(f"PAGEERROR={error}")
    expect(popup).to_be_visible(timeout=15_000)
    expect(page.locator(".composition-mark, .composition-preview").last).to_be_visible(timeout=15_000)
    expect(page.locator("[data-testid='suggestion-popup'] .suggestion button").first).to_be_visible()
    first_suggestion_text = page.locator("[data-testid='suggestion-popup'] .suggestion .suggestion-word").first
    expected_first = first_suggestion_text.inner_text()
    editor.press("ArrowDown")
    active_suggestion_text = page.locator("[data-testid='suggestion-popup'] .suggestion.active .suggestion-word").first
    expect(active_suggestion_text).to_have_text(expected_first)

    editor.press("Control+A")
    editor.type("khnhomtov")
    expect(page.locator("[data-testid='segment-preview']")).to_be_visible(timeout=15_000)
    expect(page.locator(".segment-chip").nth(0)).to_contain_text("ខ្ញុំ")
    expect(page.locator(".segment-chip").nth(1)).to_contain_text("ទៅ")
    expect(page.locator("[data-testid='suggestion-popup'] .suggestion button").first).to_contain_text("ខ្ញុំទៅ")
    editor.press("ArrowRight")
    expect(page.locator(".segment-chip.active").last).to_contain_text("ទៅ")
    expect(page.locator("[data-testid='suggestion-popup'] .suggestion button").first).to_contain_text("ទៅ")

    live_edit_button = page.locator("[data-testid='toggle-live-edit']").last
    expect(live_edit_button).to_have_class(re.compile(r".*active.*"))
    live_edit_button.click()
    expect(live_edit_button).not_to_have_class(re.compile(r".*active.*"))
    expect(page.locator("[data-testid='suggestion-popup']")).to_have_count(0)
    live_edit_button.click()
    expect(live_edit_button).to_have_class(re.compile(r".*active.*"))

    rules_button = page.locator("[data-testid='toggle-rules']").last
    rules_button.click()
    expect(rules_button).to_have_class(re.compile(r".*active.*"))
    expect(page.locator(".guide-panel")).to_be_visible(timeout=15_000)


def test_web_ui_mobile_candidate_bar_prioritizes_candidates(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    editor = mobile_page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    editor.click()
    editor.type("tverkomnaebrae")

    candidate_bar = mobile_page.locator(".candidate-bar").last
    expect(candidate_bar).to_be_visible(timeout=15_000)
    expect(mobile_page.locator("[data-testid='suggestion-popup']")).to_be_hidden()
    expect(candidate_bar.locator(".suggestion button").first).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator(".candidate-footer")).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator("[data-testid='mobile-caret-left']")).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator("[data-testid='mobile-caret-right']")).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator("[data-testid='mobile-select-up']")).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator("[data-testid='mobile-select-down']")).to_be_visible(timeout=15_000)
    expect(mobile_page.locator("[data-testid='segment-preview']")).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator(".suggestion").nth(1)).to_be_visible()


def test_web_ui_mobile_up_down_controls_cycle_candidates(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    editor = mobile_page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    editor.click()
    editor.type("preah")
    expect(mobile_page.locator(".candidate-bar").last).to_be_visible(timeout=15_000)

    active = mobile_page.locator(".candidate-bar .suggestion.active .suggestion-word").last
    mobile_page.locator("[data-testid='mobile-select-down']").last.click()
    first = active.inner_text().strip()
    assert first
    mobile_page.locator("[data-testid='mobile-select-down']").last.click()
    second = active.inner_text().strip()
    assert second
    assert second != first


def test_web_ui_mobile_caret_controls_move_cursor(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    editor = mobile_page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    assert _candidate_bar_position(mobile_page) == "sticky"
    editor.click()
    assert _candidate_bar_position(mobile_page) == "fixed"
    editor.type("preah")
    expect(mobile_page.locator(".candidate-bar").last).to_be_visible(timeout=15_000)

    assert _set_editor_caret(mobile_page, 3) == 3
    mobile_page.locator("[data-testid='mobile-caret-left']").last.click()
    assert _editor_caret(mobile_page) == 2
    mobile_page.locator("[data-testid='mobile-caret-right']").last.click()
    assert _editor_caret(mobile_page) == 3


def test_web_ui_mobile_keyboard_offset_hook_docks_candidate_bar(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    editor = mobile_page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    editor.click()
    editor.type("preah")
    expect(mobile_page.locator(".candidate-bar").last).to_be_visible(timeout=15_000)

    mobile_page.evaluate("window.__setMobileKeyboardOffsetForTest && window.__setMobileKeyboardOffsetForTest(0)")
    base_bottom = _candidate_bar_bottom_px(mobile_page)
    mobile_page.evaluate("window.__setMobileKeyboardOffsetForTest && window.__setMobileKeyboardOffsetForTest(140)")
    raised_bottom = _candidate_bar_bottom_px(mobile_page)

    assert raised_bottom >= base_bottom + 100
    expect(mobile_page.locator(".candidate-bar").last).to_be_visible(timeout=15_000)


def test_web_ui_mobile_initial_layout_keeps_candidate_strip_visible(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    editor = mobile_page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    assert mobile_page.eval_on_selector("body", "el => el.getAttribute('data-app-shell-ready')") == "1"
    assert _candidate_bar_position(mobile_page) == "sticky"
    candidate_bar = mobile_page.locator(".candidate-bar").last
    expect(candidate_bar).to_be_visible(timeout=15_000)
    expect(candidate_bar).to_have_class(re.compile(r".*candidate-bar-empty.*"))
    expect(candidate_bar.locator(".candidate-empty").last).to_be_visible(timeout=15_000)

    layout = mobile_page.evaluate(
        """() => {
            const editor = document.querySelector("[data-testid='editor-input']");
            const bar = document.querySelector(".candidate-bar");
            if (!editor || !bar) return null;
            const e = editor.getBoundingClientRect();
            const b = bar.getBoundingClientRect();
            return {
                editorHeight: e.height,
                viewportHeight: window.innerHeight,
                barHeight: b.height
            };
        }"""
    )
    assert layout is not None
    assert layout["barHeight"] > 24
    assert layout["editorHeight"] < (layout["viewportHeight"] * 0.75)


def test_web_ui_mobile_pretext_is_loaded_and_sets_layout_vars(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    mobile_page.wait_for_function("() => !!window.__pretextSizingStatus", timeout=20_000)
    loaded = mobile_page.evaluate("() => Boolean(window.__pretextSizingStatus && window.__pretextSizingStatus.loaded)")
    assert loaded is True

    css = mobile_page.evaluate(
        """() => {
            const root = document.documentElement;
            const style = getComputedStyle(root);
            return {
                footer: style.getPropertyValue("--mobile-candidate-footer-min-height").trim(),
                segment: style.getPropertyValue("--mobile-segment-min-height").trim(),
                subtitle: style.getPropertyValue("--pretext-splash-subtitle-min-height").trim()
            };
        }"""
    )
    assert css["footer"].endswith("px")
    assert css["segment"].endswith("px")
    assert css["subtitle"].endswith("px")


@pytest.mark.slow
def test_web_ui_manual_selection_lock_blocks_printable_typing(web_server: str, page) -> None:
    _goto_app(page, web_server)

    editor = page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    page.locator("[data-testid='mode-manual']").last.click()

    editor.click()
    editor.type("imsorida")
    expect(page.locator("[data-testid='suggestion-popup']").last).to_be_visible(timeout=15_000)

    editor.press("Space")
    editor.press("x")
    expect(editor).to_have_value("imsorida")


@pytest.mark.slow
def test_web_ui_manual_skip_undo_and_inline_preview_sync(web_server: str, page) -> None:
    _goto_app(page, web_server)

    editor = page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    page.locator("[data-testid='mode-manual']").last.click()

    editor.click()
    editor.type("imsorida")
    expect(page.locator("[data-testid='suggestion-popup']").last).to_be_visible(timeout=15_000)

    editor.press("Space")

    manual_preview = page.locator("[data-testid='manual-preview']")
    expect(manual_preview).to_be_visible(timeout=15_000)

    remaining_node = manual_preview.locator(".segment-chip .segment-chip-output").last
    remaining_before = remaining_node.inner_text().strip()
    editor.press("s")
    remaining_after_skip = remaining_node.inner_text().strip()
    assert remaining_after_skip != remaining_before
    assert len(remaining_after_skip) < len(remaining_before)
    expect(editor).to_have_value("imsorida")

    editor.press("u")
    expect(remaining_node).to_have_text(remaining_before)
    expect(editor).to_have_value("imsorida")

    editor.press("Enter")

    built_text_node = manual_preview.locator(".segment-chip.active .segment-chip-output").last
    expect(built_text_node).to_be_visible(timeout=15_000)
    built_text = built_text_node.inner_text().strip()
    assert built_text, "manual built text should not be empty after selecting a candidate"

    inline_preview_text = page.locator(".composition-preview .composition-preview-text").last
    expect(inline_preview_text).to_have_text(built_text)


@pytest.mark.slow
def test_web_ui_manual_sambath_shows_context_subscript_fallback(web_server: str, page) -> None:
    _goto_app(page, web_server)

    editor = page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    page.locator("[data-testid='mode-manual']").last.click()

    editor.click()
    editor.type("sambath")
    expect(page.locator("[data-testid='suggestion-popup']").last).to_be_visible(timeout=15_000)

    editor.press("Space")
    for _ in range(5):
        editor.press("s")
    _click_manual_candidate(page, "ត")

    max_cycles = 64
    for _ in range(max_cycles):
        if _manual_active_candidate_word(page) == "្ត":
            break
        editor.press("Space")
    else:
        raise AssertionError("manual context fallback candidate '្ត' did not appear while cycling")

    hint = _manual_active_candidate_hint(page)
    assert "subscript" in hint
    assert "context repeat" in hint
    assert "no-consume" in hint


@pytest.mark.slow
def test_web_ui_manual_sambath2_space_s_skips_without_text_mutation(web_server: str, page) -> None:
    _goto_app(page, web_server)

    editor = page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    page.locator("[data-testid='mode-manual']").last.click()

    editor.click()
    editor.type("sambath")
    expect(page.locator("[data-testid='suggestion-popup']").last).to_be_visible(timeout=15_000)

    editor.press("Space")
    manual_preview = page.locator("[data-testid='manual-preview']")
    expect(manual_preview).to_be_visible(timeout=15_000)
    remaining_node = manual_preview.locator(".segment-chip .segment-chip-output").last
    remaining_before = remaining_node.inner_text().strip()

    editor.press("s")

    expect(editor).to_have_value("sambath")
    remaining_after = remaining_node.inner_text().strip()
    assert remaining_after != remaining_before
    assert len(remaining_after) < len(remaining_before)
    expect(page.locator("[data-testid='suggestion-popup']").last).to_be_visible(timeout=15_000)
    expect(manual_preview).to_be_visible(timeout=15_000)
