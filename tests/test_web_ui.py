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
PORT = 4185
BASE_URL = f"http://{HOST}:{PORT}"


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


@pytest.fixture(scope="module")
def web_server():
    if _port_open(HOST, PORT):
        raise RuntimeError(f"test port {PORT} already in use")

    env = os.environ.copy()
    env["ADDR"] = HOST
    env["PORT"] = str(PORT)
    process = subprocess.Popen(
        [
            "dx",
            "serve",
            "--platform",
            "web",
            "--addr",
            HOST,
            "--port",
            str(PORT),
            "--open",
            "false",
            "--features",
            "wfst-decoder",
        ],
        cwd=ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )

    try:
        _wait_for_server(BASE_URL)
        yield BASE_URL
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def test_web_ui_suggestions_and_decoder_toggle(web_server: str) -> None:
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
        page.goto(web_server, wait_until="networkidle")

        editor = page.locator("[data-testid='editor-input']").last
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

        shadow_button = page.locator("[data-testid='decoder-shadow']").last
        shadow_button.click()
        expect(shadow_button).to_have_class(re.compile(r".*active.*"))
        expect(page.locator("[data-testid='shadow-panel']").last).to_be_visible(timeout=15_000)

        legacy_button = page.locator("[data-testid='decoder-legacy']").last
        legacy_button.click()
        expect(legacy_button).to_have_class(re.compile(r".*active.*"))

        browser.close()
