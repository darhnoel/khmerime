from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"))

from ibus_segment_preview import (  # noqa: E402
    FOCUSED_MARKER_CLOSE,
    FOCUSED_MARKER_OPEN,
    SEGMENT_SEPARATOR,
    build_segment_preview,
    build_segment_preview_fallback,
)


def test_segment_preview_format_keeps_output_input_and_separators():
    entries = [
        {"output": "ខ្ញុំ", "input": "khnhom", "focused": True},
        {"output": "ទៅ", "input": "tov", "focused": False},
        {"output": "សាលារៀន", "input": "", "focused": False},
    ]

    text, spans, focused_index = build_segment_preview(entries)

    assert text == "⟦ខ្ញុំ(khnhom)⟧ | ទៅ(tov) | សាលារៀន"
    assert focused_index == 0
    assert len(spans) == 3
    assert text[spans[0].start : spans[0].end] == "⟦ខ្ញុំ(khnhom)⟧"
    assert text[spans[1].start : spans[1].end] == "ទៅ(tov)"
    assert text[spans[2].start : spans[2].end] == "សាលារៀន"


def test_segment_preview_returns_focused_span_offsets():
    entries = [
        {"output": "ខ្ញុំ", "input": "khnhom", "focused": False},
        {"output": "ទៅ", "input": "tov", "focused": True},
    ]

    text, spans, focused_index = build_segment_preview(entries)

    assert focused_index == 1
    assert text == "ខ្ញុំ(khnhom) | ⟦ទៅ(tov)⟧"
    assert text[spans[0].start : spans[0].end] == "ខ្ញុំ(khnhom)"
    assert text[spans[1].start : spans[1].end] == "⟦ទៅ(tov)⟧"
    assert spans[1].start == spans[0].end + len(SEGMENT_SEPARATOR)


def test_segment_preview_fallback_text_ignores_invalid_entries():
    entries = [
        "invalid",
        {"output": " ", "input": "skip", "focused": False},
        {"output": "រៀន", "input": "rien", "focused": True},
    ]

    fallback_text = build_segment_preview_fallback(entries)
    assert fallback_text == "⟦រៀន(rien)⟧"
    assert fallback_text.startswith(FOCUSED_MARKER_OPEN)
    assert fallback_text.endswith(FOCUSED_MARKER_CLOSE)
