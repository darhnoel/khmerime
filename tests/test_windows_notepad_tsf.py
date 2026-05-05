import os
import subprocess
import sys

import pytest


@pytest.mark.skipif(sys.platform != "win32", reason="Windows TSF smoke test only")
@pytest.mark.skipif(
    os.environ.get("KHMERIME_RUN_WINDOWS_TSF_UI") != "1",
    reason="set KHMERIME_RUN_WINDOWS_TSF_UI=1 to run manual-assisted Notepad smoke test",
)
def test_notepad_tsf_smoke_does_not_crash():
    result = subprocess.run(
        [
            sys.executable,
            "scripts/platforms/windows/tsf/notepad_smoke.py",
            "--delay",
            os.environ.get("KHMERIME_TSF_SMOKE_DELAY", "8"),
        ],
        text=True,
        capture_output=True,
    )

    assert result.returncode == 0, result.stdout + result.stderr
