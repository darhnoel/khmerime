from pathlib import Path
from types import SimpleNamespace
import sys
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_debounced_bridge_work import DebouncedBridgeWork  # noqa: E402


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


def response_for(raw, *, ok=True, readiness="full"):
    return SimpleNamespace(
        ok=ok,
        error=None,
        readiness=readiness,
        snapshot={"raw_preedit": raw, "candidates": [raw]},
        timings={"total_ms": 0.1, "process_event_ms": 0.1},
    )


def make_work(
    *,
    call_bridge,
    apply_response,
    current_raw_preedit,
    log=lambda _message: None,
    timers=None,
    should_apply=None,
    after_apply=None,
    should_retry=None,
    slow_log_ms=999999.0,
):
    timers = timers or TimerHarness()
    return DebouncedBridgeWork(
        name="test_work",
        call_bridge=call_bridge,
        apply_response=apply_response,
        current_raw_preedit=current_raw_preedit,
        log=log,
        timeout_add=timers.timeout_add,
        source_remove=timers.source_remove,
        idle_add=lambda callback, *args: callback(*args),
        build_payload=lambda raw: {"cmd": "test_work", "raw_preedit": raw},
        min_raw_preedit_len=1,
        debounce_ms=1,
        slow_log_ms=slow_log_ms,
        should_apply=should_apply,
        after_apply=after_apply,
        should_retry=should_retry,
    )


def test_debounce_replaces_earlier_scheduled_raw_input():
    timers = TimerHarness()
    current_raw = {"value": "second"}
    calls = []
    applied = []
    work = make_work(
        timers=timers,
        call_bridge=lambda payload: calls.append(payload["raw_preedit"]) or response_for(payload["raw_preedit"]),
        apply_response=lambda response: applied.append(response.snapshot["raw_preedit"]),
        current_raw_preedit=lambda: current_raw["value"],
    )

    work.schedule("first")
    work.schedule("second")
    timers.run_latest()

    assert calls == ["second"]
    assert applied == ["second"]


def test_cancel_prevents_pending_work_from_running():
    timers = TimerHarness()
    calls = []
    work = make_work(
        timers=timers,
        call_bridge=lambda payload: calls.append(payload["raw_preedit"]) or response_for(payload["raw_preedit"]),
        apply_response=lambda _response: None,
        current_raw_preedit=lambda: "first",
    )

    work.schedule("first")
    work.cancel()

    assert timers.callbacks == {}
    assert calls == []


def test_one_running_work_keeps_latest_pending_raw_input():
    timers = TimerHarness()
    current_raw = {"value": "abcdefghij"}
    calls = []
    applied = []
    first_started = threading.Event()
    first_release = threading.Event()

    def call_bridge(payload):
        raw = payload["raw_preedit"]
        calls.append(raw)
        if raw == "abcdefghij":
            first_started.set()
            assert first_release.wait(timeout=2)
        return response_for(raw)

    work = make_work(
        timers=timers,
        call_bridge=call_bridge,
        apply_response=lambda response: applied.append(response.snapshot["raw_preedit"]),
        current_raw_preedit=lambda: current_raw["value"],
    )

    work.schedule("abcdefghij")
    timers.run_latest()
    assert first_started.wait(timeout=2)

    work.cancel()
    current_raw["value"] = "abcdefghijk"
    work.schedule("abcdefghijk")
    timers.run_latest()

    work.cancel()
    current_raw["value"] = "abcdefghijkl"
    work.schedule("abcdefghijkl")
    timers.run_latest()

    first_release.set()
    deadline = time.time() + 2
    while len(calls) < 2 and time.time() < deadline:
        time.sleep(0.01)

    assert calls == ["abcdefghij", "abcdefghijkl"]
    assert applied == ["abcdefghijkl"]


def test_stale_response_is_not_applied():
    timers = TimerHarness()
    current_raw = {"value": "abcdefghij"}
    applied = []
    logs = []
    work = make_work(
        timers=timers,
        call_bridge=lambda payload: response_for(payload["raw_preedit"]),
        apply_response=lambda response: applied.append(response.snapshot["raw_preedit"]),
        current_raw_preedit=lambda: current_raw["value"],
        log=logs.append,
    )

    work.schedule("abcdefghij")
    current_raw["value"] = "stale-target"
    timers.run_latest()

    assert applied == []
    assert any("test_work stale" in message for message in logs)


def test_slow_call_log_includes_work_name_and_raw_length():
    timers = TimerHarness()
    logs = []
    work = make_work(
        timers=timers,
        call_bridge=lambda payload: response_for(payload["raw_preedit"]),
        apply_response=lambda _response: None,
        current_raw_preedit=lambda: "abc",
        log=logs.append,
        slow_log_ms=0.0,
    )

    work.schedule("abc")
    timers.run_latest()

    assert any("test_work slow raw_len=3" in message for message in logs)


def test_adapter_retry_policy_can_reschedule_current_raw_input():
    timers = TimerHarness()
    current_raw = {"value": "abcdefghij"}
    calls = []
    applied = []
    responses = [response_for("abcdefghij", readiness="phase_a"), response_for("abcdefghij")]

    def call_bridge(payload):
        calls.append(payload["raw_preedit"])
        return responses.pop(0)

    work = make_work(
        timers=timers,
        call_bridge=call_bridge,
        apply_response=lambda response: applied.append(response.readiness),
        current_raw_preedit=lambda: current_raw["value"],
        should_retry=lambda raw, response: raw == current_raw["value"] and response.readiness == "phase_a",
    )

    work.schedule("abcdefghij")
    timers.run_latest()
    timers.run_latest()

    assert calls == ["abcdefghij", "abcdefghij"]
    assert applied == ["phase_a", "full"]
