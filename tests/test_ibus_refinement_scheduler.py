from pathlib import Path
from types import SimpleNamespace
import sys
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_refinement_scheduler import RefinementScheduler  # noqa: E402


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


def response_for(raw):
    return SimpleNamespace(
        error=None,
        readiness="full",
        snapshot={"raw_preedit": raw, "candidates": [raw]},
    )


def test_refinement_scheduler_runs_only_one_request_and_keeps_latest_pending():
    timers = TimerHarness()
    current_raw = {"value": "abcdefghij"}
    logs = []
    applied = []
    calls = []
    first_started = threading.Event()
    first_release = threading.Event()

    def call_refine(payload):
        raw = payload["raw_preedit"]
        calls.append(raw)
        if raw == "abcdefghij":
            first_started.set()
            assert first_release.wait(timeout=2)
        return response_for(raw)

    scheduler = RefinementScheduler(
        call_refine=call_refine,
        apply_response=lambda response: applied.append(response.snapshot["raw_preedit"]),
        current_raw_preedit=lambda: current_raw["value"],
        log=logs.append,
        timeout_add=timers.timeout_add,
        source_remove=timers.source_remove,
        idle_add=lambda callback, *args: callback(*args),
        debounce_ms=1,
    )

    scheduler.schedule("abcdefghij")
    timers.run_latest()
    assert first_started.wait(timeout=2)

    scheduler.cancel()
    current_raw["value"] = "abcdefghijk"
    scheduler.schedule("abcdefghijk")
    timers.run_latest()

    scheduler.cancel()
    current_raw["value"] = "abcdefghijkl"
    scheduler.schedule("abcdefghijkl")
    timers.run_latest()

    first_release.set()
    deadline = time.time() + 2
    while len(calls) < 2 and time.time() < deadline:
        time.sleep(0.01)

    assert calls == ["abcdefghij", "abcdefghijkl"]
    assert applied == ["abcdefghijkl"]


def test_refinement_scheduler_ignores_stale_response():
    timers = TimerHarness()
    current_raw = {"value": "abcdefghij"}
    applied = []

    scheduler = RefinementScheduler(
        call_refine=lambda payload: response_for(payload["raw_preedit"]),
        apply_response=lambda response: applied.append(response.snapshot["raw_preedit"]),
        current_raw_preedit=lambda: current_raw["value"],
        log=lambda _message: None,
        timeout_add=timers.timeout_add,
        source_remove=timers.source_remove,
        idle_add=lambda callback, *args: callback(*args),
        debounce_ms=1,
    )

    scheduler.schedule("abcdefghij")
    current_raw["value"] = "stale-target"
    timers.run_latest()

    assert applied == []


def test_short_word_is_scheduled_for_refinement():
    # The Word Rescuer proposes ONE whole-composition span for a single word, so the
    # words that most need it are short — `domra` (តម្រា) is five characters. The old
    # 10-char floor predates the model: it was tuned for "long composition" refinement
    # and silently starved the model of exactly the inputs it exists to handle.
    timers = TimerHarness()
    calls = []

    scheduler = RefinementScheduler(
        call_refine=lambda payload: (calls.append(payload["raw_preedit"]), response_for(payload["raw_preedit"]))[1],
        apply_response=lambda _response: None,
        current_raw_preedit=lambda: "domra",
        log=lambda _message: None,
        timeout_add=timers.timeout_add,
        source_remove=timers.source_remove,
        idle_add=lambda callback, *args: callback(*args),
        debounce_ms=1,
    )

    scheduler.schedule("domra")
    assert timers.callbacks, "a short word must still arm the debounce timer"
    timers.run_latest()

    assert calls == ["domra"], "the model never sees the word if the refine is never sent"


def test_single_character_is_not_scheduled():
    # Still don't refine on the first keystroke: one character carries no word to rescue
    # and the model's own span gate rejects it (needs >= 2 chars).
    timers = TimerHarness()
    calls = []

    scheduler = RefinementScheduler(
        call_refine=lambda payload: (calls.append(payload["raw_preedit"]), response_for(payload["raw_preedit"]))[1],
        apply_response=lambda _response: None,
        current_raw_preedit=lambda: "d",
        log=lambda _message: None,
        timeout_add=timers.timeout_add,
        source_remove=timers.source_remove,
        idle_add=lambda callback, *args: callback(*args),
        debounce_ms=1,
    )

    scheduler.schedule("d")
    assert not calls
