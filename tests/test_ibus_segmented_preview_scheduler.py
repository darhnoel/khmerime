from pathlib import Path
from types import SimpleNamespace
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_segmented_preview_scheduler import SegmentedPreviewScheduler  # noqa: E402


class TimerHarness:
    def __init__(self):
        self.callbacks = {}
        self.next_id = 1

    def timeout_add(self, _delay_ms, callback, *args):
        timeout_id = self.next_id
        self.next_id += 1
        self.callbacks[timeout_id] = (callback, args)
        return timeout_id

    def source_remove(self, timeout_id):
        self.callbacks.pop(timeout_id, None)

    def run_latest(self):
        timeout_id = max(self.callbacks)
        callback, args = self.callbacks.pop(timeout_id)
        return callback(*args)


def test_segmented_preview_scheduler_builds_refresh_payload():
    timers = TimerHarness()
    calls = []
    applied = []
    scheduler = SegmentedPreviewScheduler(
        call_bridge=lambda payload: calls.append(payload) or SimpleNamespace(ok=True, timings={}, snapshot={}),
        apply_response=lambda response: applied.append(response),
        current_raw_preedit=lambda: "khnhomttov",
        log=lambda _message: None,
        timeout_add=timers.timeout_add,
        source_remove=timers.source_remove,
        idle_add=lambda callback, *args: callback(*args),
        debounce_ms=1,
    )

    scheduler.schedule("khnhomttov")
    timers.run_latest()

    assert calls == [{"cmd": "refresh_segmented_preview", "raw_preedit": "khnhomttov"}]
    assert len(applied) == 1


def test_segmented_preview_scheduler_does_not_apply_error_response():
    timers = TimerHarness()
    applied = []
    scheduler = SegmentedPreviewScheduler(
        call_bridge=lambda _payload: SimpleNamespace(ok=False, timings={}, snapshot={}),
        apply_response=lambda response: applied.append(response),
        current_raw_preedit=lambda: "khnhomttov",
        log=lambda _message: None,
        timeout_add=timers.timeout_add,
        source_remove=timers.source_remove,
        idle_add=lambda callback, *args: callback(*args),
        debounce_ms=1,
    )

    scheduler.schedule("khnhomttov")
    timers.run_latest()

    assert applied == []
