"""Opt-in GUI smoke test: type into gedit through a live IBus daemon.

Mirrors the Windows Notepad TSF smoke test — manual-assisted, off by default, because it
needs an X session, a running ibus-daemon, and the engine installed. Bridge tests drive the
stdio protocol directly and therefore cannot catch an adapter that never renders or an engine
IBus is not routing to; this is the only check that covers that last mile.

    KHMERIME_RUN_IBUS_UI=1 python3 -m pytest tests/test_ibus_gedit_ui.py
"""

import os
import subprocess
import sys

import pytest


@pytest.mark.skipif(sys.platform != "linux", reason="Linux IBus smoke test only")
@pytest.mark.skipif(
    os.environ.get("KHMERIME_RUN_IBUS_UI") != "1",
    reason="set KHMERIME_RUN_IBUS_UI=1 to run the manual-assisted gedit smoke test",
)
def test_gedit_ibus_smoke_reaches_the_engine_and_produces_candidates():
    result = subprocess.run(
        [
            sys.executable,
            "scripts/platforms/linux/ibus/gedit_smoke.py",
            "--text",
            os.environ.get("KHMERIME_IBUS_SMOKE_TEXT", "tonle"),
        ],
        text=True,
        capture_output=True,
    )

    assert result.returncode == 0, result.stdout + result.stderr
