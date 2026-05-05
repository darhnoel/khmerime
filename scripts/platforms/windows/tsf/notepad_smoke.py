import argparse
import ctypes
import subprocess
import sys
import time
from datetime import datetime


KEYEVENTF_KEYUP = 0x0002
VK_CONTROL = 0x11
VK_RETURN = 0x0D
VK_A = 0x41
VK_C = 0x43


def key_event(vk: int, flags: int = 0) -> None:
    ctypes.windll.user32.keybd_event(vk, 0, flags, 0)


def tap(vk: int) -> None:
    key_event(vk)
    time.sleep(0.03)
    key_event(vk, KEYEVENTF_KEYUP)
    time.sleep(0.03)


def type_query(query: str) -> None:
    for ch in query:
        if ch == "\n":
            tap(VK_RETURN)
            continue
        vk = ord(ch.upper())
        if not (ord("A") <= vk <= ord("Z")):
            raise ValueError(f"Unsupported smoke-test character: {ch!r}")
        tap(vk)


def ctrl_tap(vk: int) -> None:
    key_event(VK_CONTROL)
    time.sleep(0.03)
    tap(vk)
    key_event(VK_CONTROL, KEYEVENTF_KEYUP)


def powershell(script: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script],
        text=True,
        capture_output=True,
    )


def crash_events_since(start: datetime) -> str:
    start_iso = start.isoformat()
    script = rf"""
$start = [datetime]::Parse('{start_iso}')
Get-WinEvent -FilterHashtable @{{LogName='Application'; Level=2; StartTime=$start}} -ErrorAction SilentlyContinue |
  Where-Object {{
    $_.ProviderName -eq 'Application Error' -and
    ($_.Message -like '*Notepad.exe*' -or $_.Message -like '*textinputframework.dll*' -or $_.Message -like '*msctf.dll*')
  }} |
  Select-Object TimeCreated,ProviderName,Id,Message |
  Format-List | Out-String -Width 240
"""
    result = powershell(script)
    return result.stdout.strip()


def clipboard_text() -> str:
    result = powershell("Get-Clipboard -Raw -ErrorAction SilentlyContinue")
    return result.stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser(description="Manual-assisted KhmerIME Notepad smoke test.")
    parser.add_argument("--query", default="jea\n", help="ASCII query to type; use \\n for Enter.")
    parser.add_argument("--delay", type=int, default=8, help="Seconds to wait for manual focus/input switch.")
    parser.add_argument("--post-delay", type=int, default=3, help="Seconds to wait after typing.")
    parser.add_argument("--no-clipboard", action="store_true", help="Skip Ctrl+A/C clipboard capture.")
    args = parser.parse_args()

    start = datetime.now()
    powershell("New-Item -ItemType Directory -Force C:\\Temp | Out-Null; Clear-Content C:\\Temp\\khmerime-tsf.log -ErrorAction SilentlyContinue")

    print("[khmerime] launching Notepad...")
    subprocess.Popen(["notepad.exe"])
    time.sleep(2)
    print(f"[khmerime] Click inside Notepad and switch to KhmerIME. Typing starts in {args.delay} seconds...")
    time.sleep(args.delay)

    print(f"[khmerime] typing {args.query!r}")
    type_query(args.query.encode("utf-8").decode("unicode_escape"))
    time.sleep(args.post_delay)

    events = crash_events_since(start)
    if events:
        print("[khmerime] crash events detected:")
        print(events)
        print("[khmerime] TSF log tail:")
        print(powershell("Get-Content C:\\Temp\\khmerime-tsf.log -Tail 120 -ErrorAction SilentlyContinue").stdout)
        return 1

    if not args.no_clipboard:
        ctrl_tap(VK_A)
        time.sleep(0.1)
        ctrl_tap(VK_C)
        time.sleep(0.2)
        print(f"[khmerime] clipboard text: {clipboard_text()!r}")

    print("[khmerime] no Notepad/TSF crash event detected.")
    print("[khmerime] TSF log tail:")
    print(powershell("Get-Content C:\\Temp\\khmerime-tsf.log -Tail 120 -ErrorAction SilentlyContinue").stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main())
