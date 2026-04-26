#!/usr/bin/env python3
"""IBus adapter for KhmerIME.

This process owns IBus callbacks and delegates composition logic to the
khmerime_ibus_bridge Rust binary through a JSON line protocol.
"""

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Optional, Tuple

import gi

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from ibus_segment_preview import FOCUSED_MARKER_MODE, SegmentSpan, build_segment_preview

gi.require_version("IBus", "1.0")
from gi.repository import GLib, IBus  # noqa: E402


SERVICE_NAME = "org.freedesktop.IBus.KhmerIME"
ENGINE_NAME = "khmerime"
ENGINE_LONGNAME = "KhmerIME"
ENGINE_DESCRIPTION = "Khmer romanization IME powered by KhmerIME"
ENGINE_LANGUAGE = "km"
ENGINE_LAYOUT = "us"
ENGINE_SYMBOL = "ខ"
KEY_RETURN = 0xFF0D
KEY_KP_ENTER = 0xFF8D
LOG_PATH = Path(os.environ.get("KHMERIME_IBUS_LOG", "~/.cache/khmerime/ibus_engine.log")).expanduser()
SEGMENT_PREVIEW_ALT_FOREGROUNDS = (0x2E6FD9, 0x0A8A5B)
SEGMENT_PREVIEW_FOCUSED_FOREGROUND = 0xFFFFFF
SEGMENT_PREVIEW_FOCUSED_BACKGROUND = 0x005FCC
RECOMMENDED_MARK = "✓"
DERIVED_MARK = "≈"


def log_line(message: str) -> None:
    try:
        LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
        with LOG_PATH.open("a", encoding="utf-8") as handle:
            handle.write(f"{datetime.now().isoformat()} {message}\n")
    except Exception:
        pass


@dataclass
class BridgeResponse:
    ok: bool
    consumed: bool
    commit_text: Optional[str]
    snapshot: Dict[str, Any]
    error: Optional[str]


class BridgeClient:
    def __init__(self, bridge_path: Path):
        self._proc = subprocess.Popen(
            [str(bridge_path)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

    def call(self, payload: Dict[str, Any]) -> BridgeResponse:
        if self._proc.stdin is None or self._proc.stdout is None:
            raise RuntimeError("bridge pipe is unavailable")
        self._proc.stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
        self._proc.stdin.flush()
        line = self._proc.stdout.readline()
        if not line:
            stderr = ""
            if self._proc.stderr is not None:
                stderr = self._proc.stderr.read().strip()
            raise RuntimeError(f"bridge terminated unexpectedly: {stderr}")
        data = json.loads(line)
        return BridgeResponse(
            ok=bool(data.get("ok", False)),
            consumed=bool(data.get("consumed", False)),
            commit_text=data.get("commit_text"),
            snapshot=data.get("snapshot", {}),
            error=data.get("error"),
        )

    def shutdown(self) -> None:
        try:
            self.call({"cmd": "shutdown"})
        except Exception:
            pass
        if self._proc.poll() is None:
            self._proc.terminate()
            try:
                self._proc.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                self._proc.kill()


class KhmerIMEEngine(IBus.Engine):
    def __init__(self, connection: Any, object_path: str, bridge_path: Path):
        super().__init__(connection=connection, object_path=object_path)
        self._bridge = BridgeClient(bridge_path)
        self._table = IBus.LookupTable.new(9, 0, True, True)
        self._last_preedit = ""
        self._focus_events = 0
        self._reset_events = 0
        self._last_enter_focus_events = 0
        self._last_enter_reset_events = 0
        self._capabilities = 0
        self._content_purpose = 0
        self._content_hints = 0
        self._surrounding_text = ""
        self._surrounding_cursor_pos = 0
        self._surrounding_anchor_pos = 0
        self._apply_snapshot(self._bridge.call({"cmd": "snapshot"}))
        log_line(f"engine init object_path={object_path} bridge={bridge_path}")

    def _bridge_call(self, payload: Dict[str, Any]) -> BridgeResponse:
        preedit_before = self._last_preedit
        response = self._bridge.call(payload)
        if response.commit_text:
            self.commit_text(IBus.Text.new_from_string(response.commit_text))
        self._apply_snapshot(response)

        if payload.get("cmd") == "process_key_event":
            keyval = int(payload.get("keyval", 0))
            if keyval in (KEY_RETURN, KEY_KP_ENTER):
                preedit_after = self._last_preedit
                focus_delta = self._focus_events - self._last_enter_focus_events
                reset_delta = self._reset_events - self._last_enter_reset_events
                self._last_enter_focus_events = self._focus_events
                self._last_enter_reset_events = self._reset_events
                log_line(
                    "enter_path keyval=%s keycode=%s state=%s consumed=%s commit_len=%s "
                    "preedit_before=%s preedit_after=%s focus_events=%s reset_events=%s "
                    "focus_delta=%s reset_delta=%s capabilities=%s purpose=%s hints=%s "
                    "surrounding_len=%s surrounding_cursor=%s surrounding_anchor=%s"
                    % (
                        keyval,
                        int(payload.get("keycode", 0)),
                        int(payload.get("state", 0)),
                        response.consumed,
                        len(response.commit_text or ""),
                        len(preedit_before),
                        len(preedit_after),
                        self._focus_events,
                        self._reset_events,
                        focus_delta,
                        reset_delta,
                        self._capabilities,
                        self._content_purpose,
                        self._content_hints,
                        len(self._surrounding_text),
                        self._surrounding_cursor_pos,
                        self._surrounding_anchor_pos,
                    )
                )
        if response.error:
            log_line(f"bridge error payload={payload.get('cmd')} error={response.error}")
        return response

    def _apply_snapshot(self, response: BridgeResponse) -> None:
        snapshot = response.snapshot or {}
        preedit = str(snapshot.get("preedit", ""))
        self._last_preedit = preedit
        preedit_visible = bool(preedit)
        self.update_preedit_text(IBus.Text.new_from_string(preedit), len(preedit), preedit_visible)

        candidates = snapshot.get("candidates") or []
        candidate_display = snapshot.get("candidate_display") or []
        candidate_rows = self._candidate_rows(candidates, candidate_display)
        self._table.clear()
        for candidate in candidate_rows:
            self._table.append_candidate(IBus.Text.new_from_string(str(candidate)))

        selected = snapshot.get("selected_index")
        if isinstance(selected, int):
            self._table.set_cursor_pos(selected)

        self.update_lookup_table(self._table, bool(candidate_rows))

        segment_preview = snapshot.get("segment_preview") or []
        segmented_active = bool(snapshot.get("segmented_active", False))
        if segmented_active and segment_preview:
            auxiliary_text, chunk_spans, focused_chunk_index = build_segment_preview(segment_preview)
            if auxiliary_text:
                focused_span = self._resolve_focused_chunk_span(chunk_spans, focused_chunk_index)
                focused_span_label = "none"
                focused_style_span_label = "none"
                if focused_span is not None:
                    focused_span_label = f"{focused_span.start}:{focused_span.end}"
                    padded_start, padded_end = self._focus_span_with_padding(
                        auxiliary_text, focused_span.start, focused_span.end
                    )
                    focused_style_span_label = f"{padded_start}:{padded_end}"
                render_mode = "styled"
                try:
                    auxiliary_render = self._build_styled_auxiliary_text(
                        auxiliary_text, chunk_spans, focused_chunk_index
                    )
                except Exception as err:
                    render_mode = "fallback"
                    auxiliary_render = IBus.Text.new_from_string(auxiliary_text)
                    log_line(f"segment_preview style_failed err={err}")
                self.update_auxiliary_text(auxiliary_render, True)
                self.show_auxiliary_text()
                log_line(
                    "segment_preview segmented_active=%s chunks=%s focused_chunk=%s focused_span=%s "
                    "focus_style_span=%s marker_mode=%s render=%s text_len=%s recommended=%s derived=%s"
                    % (
                        segmented_active,
                        len(chunk_spans),
                        focused_chunk_index,
                        focused_span_label,
                        focused_style_span_label,
                        FOCUSED_MARKER_MODE,
                        render_mode,
                        len(auxiliary_text),
                        sum(1 for row in candidate_display if isinstance(row, dict) and bool(row.get("recommended"))),
                        sum(
                            1
                            for row in candidate_display
                            if isinstance(row, dict)
                            and not bool(row.get("recommended"))
                            and not (row.get("roman_hints") or [])
                        ),
                    )
                )
                return
        self.hide_auxiliary_text()

    @staticmethod
    def _candidate_rows(candidates: Any, candidate_display: Any) -> list[str]:
        if not isinstance(candidates, list):
            return []

        rendered = []
        use_display = isinstance(candidate_display, list) and len(candidate_display) == len(candidates)
        for index, candidate in enumerate(candidates):
            text = str(candidate)
            if not use_display:
                rendered.append(text)
                continue

            entry = candidate_display[index]
            if not isinstance(entry, dict):
                rendered.append(text)
                continue

            output = str(entry.get("output", "")).strip() or text
            recommended = bool(entry.get("recommended", False))
            hints = [str(hint).strip() for hint in (entry.get("roman_hints") or []) if str(hint).strip()]
            label = output
            if recommended:
                label = f"{RECOMMENDED_MARK} {label}"
            elif not hints:
                label = f"{DERIVED_MARK} {label}"
            if hints:
                label = f"{label} ({' / '.join(hints[:3])})"
            rendered.append(label)
        return rendered

    @staticmethod
    def _build_styled_auxiliary_text(
        text: str, chunk_spans: list[SegmentSpan], focused_chunk_index: Optional[int]
    ) -> IBus.Text:
        rendered = IBus.Text.new_from_string(text)
        attrs = IBus.AttrList.new()
        for index, chunk in enumerate(chunk_spans):
            if chunk.end <= chunk.start:
                continue
            is_focused = chunk.focused or (focused_chunk_index is not None and index == focused_chunk_index)
            if is_focused:
                styled_start, styled_end = KhmerIMEEngine._focus_span_with_padding(text, chunk.start, chunk.end)
                attrs.append(
                    IBus.attr_foreground_new(SEGMENT_PREVIEW_FOCUSED_FOREGROUND, styled_start, styled_end)
                )
                attrs.append(
                    IBus.attr_background_new(SEGMENT_PREVIEW_FOCUSED_BACKGROUND, styled_start, styled_end)
                )
                attrs.append(IBus.attr_underline_new(IBus.AttrUnderline.DOUBLE, styled_start, styled_end))
            else:
                attrs.append(
                    IBus.attr_foreground_new(
                        SEGMENT_PREVIEW_ALT_FOREGROUNDS[index % len(SEGMENT_PREVIEW_ALT_FOREGROUNDS)],
                        chunk.start,
                        chunk.end,
                    )
                )
        rendered.set_attributes(attrs)
        return rendered

    @staticmethod
    def _resolve_focused_chunk_span(
        chunk_spans: list[SegmentSpan], focused_chunk_index: Optional[int]
    ) -> Optional[SegmentSpan]:
        if focused_chunk_index is not None and 0 <= focused_chunk_index < len(chunk_spans):
            return chunk_spans[focused_chunk_index]
        for chunk in chunk_spans:
            if chunk.focused:
                return chunk
        return None

    @staticmethod
    def _focus_span_with_padding(text: str, start: int, end: int) -> Tuple[int, int]:
        padded_start = start
        padded_end = end
        if padded_start > 0 and text[padded_start - 1].isspace():
            padded_start -= 1
        if padded_end < len(text) and text[padded_end].isspace():
            padded_end += 1
        return padded_start, padded_end

    def do_process_key_event(self, keyval: int, keycode: int, state: int) -> bool:
        try:
            response = self._bridge_call(
                {
                    "cmd": "process_key_event",
                    "keyval": int(keyval),
                    "keycode": int(keycode),
                    "state": int(state),
                }
            )
            log_line(
                "key_event keyval=%s keycode=%s state=%s consumed=%s preedit=%r cand=%s"
                % (
                    keyval,
                    keycode,
                    state,
                    response.consumed,
                    str(response.snapshot.get("preedit", "")),
                    len(response.snapshot.get("candidates", []) or []),
                )
            )
            return response.consumed
        except Exception as err:
            log_line(f"process_key_event failed keyval={keyval} keycode={keycode} state={state} err={err}")
            return False

    def do_focus_in(self) -> None:
        self._focus_events += 1
        log_line(f"focus_in count={self._focus_events}")
        self._bridge_call({"cmd": "focus_in"})

    def do_focus_in_id(self, object_path: str, client: str) -> None:
        log_line(f"focus_in_id object_path={object_path} client={client}")
        self.do_focus_in()

    def do_focus_out(self) -> None:
        self._focus_events += 1
        log_line(f"focus_out count={self._focus_events}")
        self._bridge_call({"cmd": "focus_out"})

    def do_focus_out_id(self, object_path: str) -> None:
        log_line(f"focus_out_id object_path={object_path}")
        self.do_focus_out()

    def do_reset(self) -> None:
        self._reset_events += 1
        log_line(f"reset count={self._reset_events}")
        self._bridge_call({"cmd": "reset"})

    def do_enable(self) -> None:
        log_line("enable")
        self._bridge_call({"cmd": "enable"})
        self.do_focus_in()

    def do_disable(self) -> None:
        log_line("disable")
        self._bridge_call({"cmd": "disable"})

    def do_set_cursor_location(self, x: int, y: int, width: int, height: int) -> None:
        self._bridge_call(
            {
                "cmd": "set_cursor_location",
                "x": int(x),
                "y": int(y),
                "width": int(width),
                "height": int(height),
            }
        )

    def do_set_capabilities(self, cap: int) -> None:
        self._capabilities = int(cap)
        log_line(f"set_capabilities cap={self._capabilities}")

    def do_set_content_type(self, purpose: int, hints: int) -> None:
        self._content_purpose = int(purpose)
        self._content_hints = int(hints)
        log_line(f"set_content_type purpose={self._content_purpose} hints={self._content_hints}")

    def do_set_surrounding_text(self, text: Any, cursor_pos: int, anchor_pos: int) -> None:
        content = ""
        if isinstance(text, IBus.Text):
            content = text.get_text()
        elif text is not None:
            content = str(text)
        self._surrounding_text = content
        self._surrounding_cursor_pos = int(cursor_pos)
        self._surrounding_anchor_pos = int(anchor_pos)
        log_line(
            "set_surrounding_text len=%s cursor=%s anchor=%s"
            % (len(self._surrounding_text), self._surrounding_cursor_pos, self._surrounding_anchor_pos)
        )

    def do_destroy(self) -> None:
        self._bridge.shutdown()
        super().do_destroy()


class KhmerIMEFactory(IBus.Factory):
    def __init__(self, bus: IBus.Bus, bridge_path: Path):
        super().__init__(connection=bus.get_connection(), object_path=IBus.PATH_FACTORY)
        self._bridge_path = bridge_path
        self._engine_id = 0

    def do_create_engine(self, engine_name: str) -> IBus.Engine:
        if engine_name != ENGINE_NAME:
            log_line(f"create_engine unexpected engine_name={engine_name}")
            raise RuntimeError(f"unexpected engine name: {engine_name}")
        object_path = f"/org/freedesktop/IBus/KhmerIME/Engine/{self._engine_id}"
        self._engine_id += 1
        log_line(f"create_engine engine_name={engine_name} object_path={object_path}")
        return KhmerIMEEngine(
            connection=self.get_connection(),
            object_path=object_path,
            bridge_path=self._bridge_path,
        )


def component_xml(exec_path: Path) -> str:
    exec_cmd = f"{exec_path} --ibus"
    return f"""<component>
    <name>{SERVICE_NAME}</name>
    <description>KhmerIME input method engine</description>
    <version>0.1.0</version>
    <license>MIT</license>
    <author>KhmerIME contributors</author>
    <homepage>https://github.com/darhnoel/khmerime</homepage>
    <textdomain>khmerime</textdomain>
    <exec>{exec_cmd}</exec>
    <engines>
        <engine>
            <name>{ENGINE_NAME}</name>
            <longname>{ENGINE_LONGNAME}</longname>
            <description>{ENGINE_DESCRIPTION}</description>
            <language>{ENGINE_LANGUAGE}</language>
            <license>MIT</license>
            <author>KhmerIME contributors</author>
            <icon></icon>
            <layout>{ENGINE_LAYOUT}</layout>
            <symbol>{ENGINE_SYMBOL}</symbol>
        </engine>
    </engines>
</component>"""


def register_component(bus: IBus.Bus, exec_path: Path) -> None:
    component = IBus.Component.new(
        SERVICE_NAME,
        "KhmerIME input method engine",
        "0.1.0",
        "MIT",
        "KhmerIME contributors",
        "https://github.com/darhnoel/khmerime",
        "khmerime",
        str(exec_path),
    )
    engine = IBus.EngineDesc.new(
        ENGINE_NAME,
        ENGINE_LONGNAME,
        ENGINE_DESCRIPTION,
        ENGINE_LANGUAGE,
        "MIT",
        "KhmerIME contributors",
        "",
        ENGINE_LAYOUT,
    )
    component.add_engine(engine)
    bus.register_component(component)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="KhmerIME IBus adapter")
    parser.add_argument("--ibus", action="store_true", help="Run as IBus-launched engine")
    parser.add_argument("--xml", action="store_true", help="Print component XML and exit")
    parser.add_argument(
        "--bridge-path",
        default=os.environ.get("KHMERIME_IBUS_BRIDGE", ""),
        help="Path to khmerime_ibus_bridge binary",
    )
    return parser.parse_args()


def resolve_bridge_path(raw_path: str) -> Path:
    if raw_path:
        return Path(raw_path).expanduser().resolve()
    current = Path(__file__).resolve()
    sibling = current.with_name("khmerime-ibus-bridge")
    if sibling.exists():
        return sibling
    return Path("khmerime_ibus_bridge")


def main() -> int:
    args = parse_args()
    exec_path = Path(__file__).resolve()
    if args.xml:
        print(component_xml(exec_path))
        return 0

    IBus.init()
    bus = IBus.Bus()
    bridge_path = resolve_bridge_path(args.bridge_path)
    log_line(f"startup ibus={args.ibus} bridge={bridge_path}")
    factory = KhmerIMEFactory(bus, bridge_path)

    loop = GLib.MainLoop()

    def on_disconnected(_: Any) -> None:
        loop.quit()

    bus.connect("disconnected", on_disconnected)

    if args.ibus:
        bus.request_name(SERVICE_NAME, 0)
        log_line(f"requested name {SERVICE_NAME}")
    else:
        register_component(bus, exec_path)
        log_line("registered component for non-ibus launch")

    signal.signal(signal.SIGTERM, lambda *_: loop.quit())
    signal.signal(signal.SIGINT, lambda *_: loop.quit())
    loop.run()
    factory.destroy()
    return 0


if __name__ == "__main__":
    sys.exit(main())
