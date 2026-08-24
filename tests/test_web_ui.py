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
        ["bash", "scripts/web/serve_phone.sh"],
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
    expect(page.locator(".composition-mark").last).to_be_visible(timeout=15_000)
    expect(page.locator(".composition-preview")).to_have_count(0)
    expect(page.locator("[data-testid='suggestion-popup'] .suggestion button").first).to_be_visible()
    first_suggestion_text = page.locator("[data-testid='suggestion-popup'] .suggestion .suggestion-word").first
    expected_first = first_suggestion_text.inner_text()
    editor.press("ArrowDown")
    active_suggestion_text = page.locator("[data-testid='suggestion-popup'] .suggestion.active .suggestion-word").first
    expect(active_suggestion_text).to_have_text(expected_first)

    editor.press("Control+A")
    editor.type("khnhomtov")
    expect(popup).to_have_attribute("data-candidate-level", "phrase", timeout=15_000)
    expect(page.locator("[data-testid='segment-preview']")).to_have_count(0)
    expect(editor).to_have_value("khnhomtov")
    expect(page.locator(".composition-preview")).to_have_count(0)
    expect(popup.locator(".candidate-choice").first).to_contain_text("ខ្ញុំទៅ")

    # Phrase mode consumes horizontal arrows and never jumps into word-level
    # suggestions. Segment Edit is an explicit Tab transition.
    editor.press("ArrowRight")
    expect(popup).to_have_attribute("data-candidate-level", "phrase")
    editor.press("Tab")
    expect(popup).to_have_attribute("data-candidate-level", "segment")
    expect(popup.locator("[data-testid='segment-edit-header']")).to_be_visible()
    expect(popup.locator(".candidate-segment.active")).to_contain_text("ខ្ញុំ")
    expect(popup.locator(".candidate-choice").first).to_contain_text("ខ្ញុំ")
    editor.press("ArrowRight")
    expect(popup.locator(".candidate-segment.active")).to_contain_text("ទៅ")
    expect(popup.locator(".candidate-choice").first).to_contain_text("ទៅ")
    editor.press("Tab")
    expect(popup).to_have_attribute("data-candidate-level", "phrase")

    live_edit_button = page.locator("[data-testid='toggle-live-edit']").last
    expect(live_edit_button).to_have_class(re.compile(r".*active.*"))
    live_edit_button.click()
    expect(live_edit_button).not_to_have_class(re.compile(r".*active.*"))
    expect(page.locator("[data-testid='suggestion-popup']")).to_have_count(0)
    live_edit_button.click()
    expect(live_edit_button).to_have_class(re.compile(r".*active.*"))

def test_web_ui_mobile_keeps_caret_candidate_surface(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    editor = mobile_page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    editor.click()
    editor.type("tverkomnaebrae")

    candidate_bar = mobile_page.locator(".candidate-bar").last
    popup = mobile_page.locator("[data-testid='suggestion-popup']").last
    expect(candidate_bar).to_be_visible(timeout=15_000)
    expect(popup).to_be_visible(timeout=15_000)
    expect(popup.locator(".candidate-choice").first).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator(".candidate-footer")).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator("[data-testid='mobile-caret-left']")).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator("[data-testid='mobile-caret-right']")).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator("[data-testid='mobile-select-up']")).to_be_visible(timeout=15_000)
    expect(candidate_bar.locator("[data-testid='mobile-select-down']")).to_be_visible(timeout=15_000)
    expect(mobile_page.locator("[data-testid='segment-preview']")).to_have_count(0)
    expect(mobile_page.locator(".candidate-track-mobile .suggestion")).to_have_count(0)


def test_web_ui_exact_gumnit_stays_flat_and_ranks_correct_spelling_first(web_server: str, page) -> None:
    _goto_app(page, web_server)

    editor = page.locator("[data-testid='editor-input']").last
    editor.click()
    editor.type("gumnit")

    popup = page.locator("[data-testid='suggestion-popup']").last
    expect(popup).to_be_visible(timeout=15_000)
    expect(popup).to_have_attribute("data-candidate-level", "flat")
    expect(popup.locator(".candidate-choice .suggestion-word").first).to_have_text("គំនិត")


def test_web_ui_mobile_rules_open_as_full_width_page(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    mobile_page.locator("[data-testid='toggle-sidebar']").last.click()
    rules_button = mobile_page.locator("[data-testid='toggle-rules']").last
    rules_button.click()
    guide = mobile_page.locator("[data-testid='guide-sheet']").last
    expect(guide).to_be_visible(timeout=15_000)

    viewport_width = mobile_page.evaluate("window.innerWidth")
    assert guide.bounding_box()["width"] == viewport_width
    assert guide.bounding_box()["height"] == mobile_page.evaluate("window.innerHeight")
    assert guide.locator(".guide-scroll").evaluate("node => node.scrollHeight > node.clientHeight")
    expect(guide.locator(".guide-scroll")).to_have_css("scrollbar-width", "none")
    expect(guide.locator(".guide-overflow-cue")).to_have_count(0)

    mobile_page.keyboard.press("Escape")
    expect(mobile_page.locator("[data-testid='guide-sheet']")).to_have_count(0)


def test_web_ui_rules_open_as_dedicated_page(web_server: str, page) -> None:
    _goto_app(page, web_server)

    rules_button = page.locator("[data-testid='toggle-rules']").last
    rules_button.click()
    guide = page.locator("[data-testid='guide-sheet']").last
    expect(guide).to_be_visible(timeout=15_000)
    expect(guide).to_have_css("position", "fixed")
    assert guide.bounding_box()["width"] == page.evaluate("window.innerWidth")
    assert guide.bounding_box()["height"] == page.evaluate("window.innerHeight")

    page.keyboard.press("Escape")
    expect(guide).to_have_count(0)
    expect(rules_button).not_to_have_class(re.compile(r".*active.*"))

    rules_button.click()
    expect(page.locator("[data-testid='guide-sheet']").last).to_be_visible(timeout=15_000)
    page.locator("[data-testid='close-rules']").last.click()
    expect(page.locator("[data-testid='guide-sheet']")).to_have_count(0)


def test_web_ui_mobile_up_down_controls_cycle_candidates(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    editor = mobile_page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    editor.click()
    editor.type("preah")
    expect(mobile_page.locator(".candidate-bar").last).to_be_visible(timeout=15_000)

    active = mobile_page.locator("[data-testid='suggestion-popup'] .suggestion.active .suggestion-word").last
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


def test_web_ui_digit_shortcut_selects_candidate_without_commit(web_server: str, page) -> None:
    _goto_app(page, web_server)

    editor = page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    editor.click()
    editor.type("jea")

    popup = page.locator("[data-testid='suggestion-popup']").last
    expect(popup).to_be_visible(timeout=15_000)

    first_suggestion_text = popup.locator(".suggestion .suggestion-word").first.inner_text().strip()
    assert first_suggestion_text

    editor.press("1")

    active_suggestion_text = popup.locator(".suggestion.active .suggestion-word").first
    expect(active_suggestion_text).to_have_text(first_suggestion_text)
    expect(editor).to_have_value("jea")


def test_web_ui_mobile_initial_layout_keeps_candidate_strip_visible(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    editor = mobile_page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    assert mobile_page.eval_on_selector("body", "el => el.getAttribute('data-app-shell-ready')") == "1"
    assert _candidate_bar_position(mobile_page) == "sticky"
    candidate_bar = mobile_page.locator(".candidate-bar").last
    expect(candidate_bar).to_be_visible(timeout=15_000)
    expect(candidate_bar).to_have_class(re.compile(r".*candidate-bar-empty.*"))
    expect(candidate_bar.locator("[data-testid='mobile-candidate-hints']").last).to_be_visible(timeout=15_000)

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
    assert layout["editorHeight"] >= (layout["viewportHeight"] * 0.55)


@pytest.mark.slow
def test_next_word_predictions_use_only_the_dock_and_enter_inserts_newline(web_server: str, page) -> None:
    _goto_app(page, web_server)

    editor = page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    editor.fill("ខ្ញុំ ")

    dock = page.locator("[data-testid='next-word-dock']").last
    expect(dock).to_be_visible(timeout=15_000)
    expect(page.locator("[data-testid='suggestion-popup']")).to_have_count(0)
    expect(page.locator(".candidate-track-mobile .suggestion")).to_have_count(0)
    expect(page.locator("[data-testid='candidate-hints']")).to_contain_text("Enter")

    editor.press("Enter")
    expect(editor).to_have_value("ខ្ញុំ \n")


@pytest.mark.slow
def test_mobile_next_word_predictions_are_not_duplicated_in_candidate_track(web_server: str, mobile_page) -> None:
    _goto_app(mobile_page, web_server)

    editor = mobile_page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    editor.fill("ខ្ញុំ ")

    expect(mobile_page.locator("[data-testid='next-word-dock']").last).to_be_visible(timeout=15_000)
    expect(mobile_page.locator(".candidate-track-mobile .suggestion")).to_have_count(0)
    expect(mobile_page.locator("[data-testid='mobile-candidate-hints']")).to_contain_text("Enter")


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
def test_web_ui_add_pair_flick_save_and_normal_decode(web_server: str, page) -> None:
    _goto_app(page, web_server)

    editor = page.locator("[data-testid='editor-input']").last
    expect(editor).to_be_visible(timeout=20_000)
    expect(page.locator("[data-testid='engine-status']")).to_have_count(0, timeout=20_000)
    page.locator("[data-testid='toggle-saved-mappings']").last.click()
    expect(page.locator("[data-testid='saved-words-page']")).to_be_visible()
    page.locator("[data-testid='add-saved-mapping']").last.click()

    modal = page.locator("[data-testid='add-pair-modal']")
    expect(modal).to_be_visible()
    roman_field = page.locator("[data-testid='add-pair-roman']")
    khmer_field = page.locator("[data-testid='add-pair-khmer']")
    expect(roman_field).to_have_attribute("placeholder", "ឧ. khnhom")
    for field in (roman_field, khmer_field):
        weight = int(field.evaluate("el => getComputedStyle(el).fontWeight"))
        assert weight <= 500, "placeholder fields must not inherit the label's bold weight"
    keyboard = page.locator("[data-testid='flick-keyboard']")
    expect(keyboard).to_be_visible()
    expect(editor).to_have_value("")
    expect(page.locator("[data-testid='suggestion-popup']")).to_have_count(0)
    roman_field.fill("zzqname")

    center_key = page.locator("[data-testid='flick-key-1-1']")
    center_key.click()
    expect(khmer_field).to_have_value("ក")
    expect(editor).to_have_value("")

    box = center_key.bounding_box()
    assert box is not None
    x = box["x"] + box["width"] / 2
    y = box["y"] + box["height"] / 2
    page.mouse.move(x, y)
    page.mouse.down()
    expect(page.locator(".flick-preview")).to_be_visible()
    page.mouse.move(x, y + 24)
    page.mouse.up()
    expect(khmer_field).to_have_value("កគ")
    page.locator("[data-testid='flick-backspace']").click()
    expect(khmer_field).to_have_value("ក")

    page.locator("[data-testid='add-pair-save']").click()
    expect(modal).to_have_count(0)
    row = page.locator(".saved-words-row")
    expect(row).to_contain_text("zzqname")
    expect(row).to_contain_text("ក")

    search = page.locator("[data-testid='saved-words-search']")
    search.fill("missing")
    expect(page.locator(".saved-words-row")).to_have_count(0)
    search.fill("ក")
    expect(page.locator(".saved-words-row")).to_have_count(1)

    page.locator("[data-testid='saved-word-menu-button']").click()
    page.locator("[data-testid='edit-saved-word']").click()
    expect(page.locator("[data-testid='add-pair-roman']")).to_have_value("zzqname")
    expect(page.locator("[data-testid='add-pair-khmer']")).to_have_value("ក")
    page.locator("[data-testid='add-pair-cancel']").click()

    page.locator("[data-testid='saved-word-menu-button']").click()
    page.locator("[data-testid='delete-saved-word']").click()
    expect(page.locator(".saved-words-row")).to_have_count(0)
    expect(page.locator("[data-testid='saved-words-toast']")).to_be_visible()
    page.locator("[data-testid='undo-delete-saved-word']").click()
    expect(page.locator(".saved-words-row")).to_have_count(1)

    page.locator("[data-testid='saved-words-back']").click()

    editor.fill("zzqname")
    popup = page.locator("[data-testid='suggestion-popup']").last
    expect(popup).to_be_visible(timeout=15_000)
    expect(popup.locator(".suggestion-word").first).to_have_text("ក")


@pytest.mark.slow
def test_web_ui_flick_keyboard_treats_composed_key_as_one_backspace_unit(web_server: str, page) -> None:
    _goto_app(page, web_server)

    editor = page.locator("[data-testid='editor-input']").last
    page.locator("[data-testid='toggle-saved-mappings']").last.click()
    page.locator("[data-testid='add-saved-mapping']").last.click()

    page.locator("[data-testid='flick-key-3-1']").click()
    expect(page.locator("[data-testid='add-pair-khmer']")).to_have_value("ឲ្យ")
    expect(editor).to_have_value("")
    page.locator("[data-testid='flick-backspace']").click()
    expect(page.locator("[data-testid='add-pair-khmer']")).to_have_value("")
    expect(page.locator("[data-testid='flick-space']")).to_have_count(0)
    expect(page.locator("[data-testid='flick-enter']")).to_have_count(0)
