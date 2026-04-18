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
    try:
        page.goto(url, wait_until="domcontentloaded", timeout=60_000)
    except PlaywrightTimeoutError:
        page.goto(url, wait_until="commit", timeout=60_000)


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
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
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


def test_web_ui_desktop_popup_and_live_edit_toggle(web_server: str) -> None:
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        page = browser.new_page()
        console_messages = []
        page_errors = []
        page.on("console", lambda msg: console_messages.append(f"{msg.type}: {msg.text}"))
        page.on("pageerror", lambda exc: page_errors.append(str(exc)))
        page.add_init_script(
            """
            window.localStorage.clear();
            window.sessionStorage.clear();
            """
        )
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
        active_suggestion_text = page.locator(
            "[data-testid='suggestion-popup'] .suggestion.active .suggestion-word"
        ).first
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

        browser.close()


def test_web_ui_mobile_candidate_bar_prioritizes_candidates(web_server: str) -> None:
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        page = browser.new_page(viewport={"width": 390, "height": 844})
        page.add_init_script(
            """
            window.localStorage.clear();
            window.sessionStorage.clear();
            """
        )
        _goto_app(page, web_server)

        editor = page.locator("[data-testid='editor-input']").last
        expect(editor).to_be_visible(timeout=20_000)
        editor.click()
        editor.type("tverkomnaebrae")

        candidate_bar = page.locator(".candidate-bar").last
        expect(candidate_bar).to_be_visible(timeout=15_000)
        expect(page.locator("[data-testid='suggestion-popup']")).to_be_hidden()
        expect(candidate_bar.locator(".suggestion button").first).to_be_visible(timeout=15_000)
        expect(candidate_bar.locator(".candidate-footer")).to_be_hidden()
        expect(page.locator("[data-testid='segment-preview']")).to_be_hidden()
        expect(candidate_bar.locator(".suggestion").nth(1)).to_be_visible()

        browser.close()


def test_web_ui_manual_selection_lock_blocks_printable_typing(web_server: str) -> None:
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        page = browser.new_page()
        page.add_init_script(
            """
            window.localStorage.clear();
            window.sessionStorage.clear();
            """
        )
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

        browser.close()


def test_web_ui_manual_skip_undo_and_inline_preview_sync(web_server: str) -> None:
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        page = browser.new_page()
        page.add_init_script(
            """
            window.localStorage.clear();
            window.sessionStorage.clear();
            """
        )
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

        browser.close()


def test_web_ui_manual_sambath_shows_context_subscript_fallback(web_server: str) -> None:
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        page = browser.new_page()
        page.add_init_script(
            """
            window.localStorage.clear();
            window.sessionStorage.clear();
            """
        )
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

        browser.close()


def test_web_ui_manual_sambath2_space_s_skips_without_text_mutation(web_server: str) -> None:
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        page = browser.new_page()
        page.add_init_script(
            """
            window.localStorage.clear();
            window.sessionStorage.clear();
            """
        )
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

        browser.close()
