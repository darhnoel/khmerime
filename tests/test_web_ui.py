import os
import re
import socket
import subprocess
import time
from pathlib import Path
from urllib.request import urlopen

import pytest
from playwright.sync_api import expect, sync_playwright


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


def _wait_for_shell_splash(url: str, timeout_s: float = 20.0) -> None:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            with urlopen(url, timeout=2.0) as response:
                html = response.read().decode("utf-8", errors="replace")
            if 'data-testid="preboot-splash"' in html:
                return
        except Exception:
            pass
        time.sleep(0.25)
    raise RuntimeError("preboot splash was not injected into index.html")


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
        _wait_for_shell_splash(base_url)
        yield base_url
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def test_web_ui_suggestions_and_live_edit_toggle(web_server: str) -> None:
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
        page.goto(web_server, wait_until="domcontentloaded")
        expect(page.locator("[data-testid='preboot-splash']")).to_have_count(1)
        page.wait_for_load_state("networkidle")

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
