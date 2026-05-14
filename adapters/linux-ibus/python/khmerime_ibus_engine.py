#!/usr/bin/env python3
"""IBus adapter for KhmerIME.

This process owns IBus callbacks and delegates composition logic to the
khmerime_ibus_bridge Rust binary through a JSON line protocol.
"""

from __future__ import annotations

import argparse
import os
import signal
import sys
import threading
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Optional

import gi

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from ibus_bridge_client import BridgeClient, BridgeResponse
from ibus_candidate_render import candidate_rows
from ibus_component import ENGINE_NAME, ENGINE_NIDA_NAME, SERVICE_NAME, component_xml, register_component
from ibus_refinement import RefinementScheduler
from ibus_segment_preview import (
    FOCUSED_MARKER_MODE,
    SegmentSpan,
    build_segment_preview,
    focused_raw_input_span,
)

try:
    gi.require_version("Gdk", "3.0")
    from gi.repository import Gdk  # type: ignore[attr-defined]
except Exception:
    Gdk = None  # type: ignore[assignment]

gi.require_version("IBus", "1.0")
from gi.repository import GLib, IBus  # noqa: E402


KEY_RETURN = 0xFF0D
KEY_KP_ENTER = 0xFF8D
KEY_CAPS_LOCK = 0xFFE5
STATE_LOCK_MASK = int(IBus.ModifierType.LOCK_MASK)
STATE_RELEASE_MASK = int(IBus.ModifierType.RELEASE_MASK)
MODE_PROPERTY_KEY = "InputMode"
MODE_PROPERTY_ROMAN_KEY = "InputMode.Roman"
MODE_PROPERTY_NIDA_KEY = "InputMode.NIDA"
MODE_ROMAN_SYMBOL = "R"
MODE_NIDA_SYMBOL = "ខ"
MODE_ROMAN_LABEL = "Roman"
MODE_NIDA_LABEL = "NIDA"
MODE_TOOLTIP = "Toggle KhmerIME Roman/NIDA mode"
LOG_PATH = Path(os.environ.get("KHMERIME_IBUS_LOG", "~/.cache/khmerime/ibus_engine.log")).expanduser()


def log_line(message: str) -> None:
    try:
        LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
        with LOG_PATH.open("a", encoding="utf-8") as handle:
            handle.write(f"{datetime.now().isoformat()} {message}\n")
    except Exception:
        pass


class KhmerIMEEngine(IBus.Engine):
    def __init__(self, connection: Any, object_path: str, bridge_path: Path, initial_input_mode: str = "roman"):
        super().__init__(connection=connection, object_path=object_path)
        self._bridge = BridgeClient(bridge_path, initial_input_mode=initial_input_mode)
        self._bridge_lock = threading.Lock()
        self._table = IBus.LookupTable.new(9, 0, True, True)
        self._last_preedit = ""
        self._last_raw_preedit = ""
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
        self._mode_property_registered = False
        self._mode_main_prop: Optional[IBus.Property] = None
        self._mode_sub_props: Dict[str, IBus.Property] = {}
        self._mode_main_prop_list: Optional[IBus.PropList] = None
        self._current_input_mode = "roman"
        self._pending_caps_input_mode: Optional[str] = None
        self._refinement = RefinementScheduler(
            call_refine=self._call_bridge_raw,
            apply_response=self._apply_response,
            current_raw_preedit=lambda: self._last_raw_preedit,
            log=log_line,
            timeout_add=GLib.timeout_add,
            source_remove=GLib.source_remove,
            idle_add=GLib.idle_add,
        )
        self._current_input_mode = "nida" if initial_input_mode == "nida" else "roman"
        self._register_mode_property(self._current_input_mode)
        self._apply_snapshot(self._call_bridge_raw({"cmd": "snapshot"}))
        log_line(f"engine init object_path={object_path} bridge={bridge_path} input_mode={initial_input_mode}")

    def _call_bridge_raw(self, payload: Dict[str, Any]) -> BridgeResponse:
        with self._bridge_lock:
            return self._bridge.call(payload)

    def _bridge_call(self, payload: Dict[str, Any]) -> BridgeResponse:
        preedit_before = self._last_preedit
        response = self._call_bridge_raw(payload)
        self._apply_response(response)

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

    def _apply_response(self, response: BridgeResponse) -> None:
        if not response.ok:
            return
        if response.commit_text:
            self._clear_composition_ui_for_commit(response)
            self.commit_text(IBus.Text.new_from_string(response.commit_text))
            return
        self._apply_snapshot(response)

    def _apply_snapshot(self, response: BridgeResponse) -> None:
        snapshot = response.snapshot or {}
        self._update_mode_property(str(snapshot.get("input_mode", "roman")))
        raw_preedit = str(snapshot.get("raw_preedit", ""))
        render_preedit = raw_preedit or str(snapshot.get("preedit", ""))
        self._last_preedit = render_preedit
        self._last_raw_preedit = raw_preedit
        preedit_visible = bool(render_preedit)
        segment_preview = snapshot.get("segment_preview") or []
        segmented_active = bool(snapshot.get("segmented_active", False))
        preedit_text = IBus.Text.new_from_string(render_preedit)
        if segmented_active:
            focused_raw_span = focused_raw_input_span(
                raw_preedit,
                segment_preview,
                snapshot.get("focused_segment_index"),
            )
            if focused_raw_span is not None:
                start, end = focused_raw_span
                attrs = IBus.AttrList.new()
                attrs.append(IBus.attr_underline_new(IBus.AttrUnderline.SINGLE, start, end))
                preedit_text.set_attributes(attrs)
        self.update_preedit_text(
            preedit_text,
            len(render_preedit),
            preedit_visible,
        )

        candidates = snapshot.get("candidates") or []
        candidate_display = snapshot.get("candidate_display") or []
        rendered_candidates = candidate_rows(candidates, candidate_display)
        self._table.clear()
        for candidate in rendered_candidates:
            self._table.append_candidate(IBus.Text.new_from_string(str(candidate)))

        selected = snapshot.get("selected_index")
        if isinstance(selected, int) and rendered_candidates:
            self._table.set_cursor_pos(min(selected, len(rendered_candidates) - 1))

        self.update_lookup_table(self._table, bool(rendered_candidates))

        if segmented_active and segment_preview:
            auxiliary_text, chunk_spans, focused_chunk_index = build_segment_preview(segment_preview)
            if auxiliary_text:
                focused_span = self._resolve_focused_chunk_span(chunk_spans, focused_chunk_index)
                focused_span_label = "none"
                if focused_span is not None:
                    focused_span_label = f"{focused_span.start}:{focused_span.end}"
                auxiliary_render = IBus.Text.new_from_string(auxiliary_text)
                self.update_auxiliary_text(auxiliary_render, True)
                self.show_auxiliary_text()
                log_line(
                    "segment_preview segmented_active=%s chunks=%s focused_chunk=%s focused_span=%s "
                    "marker_mode=%s render=marker text_len=%s recommended=%s derived=%s"
                    % (
                        segmented_active,
                        len(chunk_spans),
                        focused_chunk_index,
                        focused_span_label,
                        FOCUSED_MARKER_MODE,
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
    def _normalize_mode(mode: str) -> str:
        return "nida" if mode == "nida" else "roman"

    def _build_mode_property(self, mode: str) -> IBus.Property:
        normalized = self._normalize_mode(mode)
        active_label = MODE_NIDA_LABEL if normalized == "nida" else MODE_ROMAN_LABEL
        active_symbol = MODE_NIDA_SYMBOL if normalized == "nida" else MODE_ROMAN_SYMBOL

        sub_props = IBus.PropList.new()
        roman_sub = IBus.Property.new(
            MODE_PROPERTY_ROMAN_KEY,
            IBus.PropType.RADIO,
            IBus.Text.new_from_string(MODE_ROMAN_LABEL),
            "",
            IBus.Text.new_from_string("KhmerIME romanization input"),
            True,
            True,
            IBus.PropState.CHECKED if normalized == "roman" else IBus.PropState.UNCHECKED,
            None,
        )
        roman_sub.set_symbol(IBus.Text.new_from_string(MODE_ROMAN_SYMBOL))
        nida_sub = IBus.Property.new(
            MODE_PROPERTY_NIDA_KEY,
            IBus.PropType.RADIO,
            IBus.Text.new_from_string(MODE_NIDA_LABEL),
            "",
            IBus.Text.new_from_string("KhmerIME direct NIDA input"),
            True,
            True,
            IBus.PropState.CHECKED if normalized == "nida" else IBus.PropState.UNCHECKED,
            None,
        )
        nida_sub.set_symbol(IBus.Text.new_from_string(MODE_NIDA_SYMBOL))
        sub_props.append(roman_sub)
        sub_props.append(nida_sub)
        self._mode_sub_props = {
            MODE_PROPERTY_ROMAN_KEY: roman_sub,
            MODE_PROPERTY_NIDA_KEY: nida_sub,
        }

        main_prop = IBus.Property.new(
            MODE_PROPERTY_KEY,
            IBus.PropType.MENU,
            IBus.Text.new_from_string(active_label),
            "",
            IBus.Text.new_from_string(MODE_TOOLTIP),
            True,
            True,
            IBus.PropState.UNCHECKED,
            sub_props,
        )
        main_prop.set_symbol(IBus.Text.new_from_string(active_symbol))
        return main_prop

    def _register_mode_property(self, mode: str) -> None:
        normalized = self._normalize_mode(mode)
        self._mode_main_prop = self._build_mode_property(normalized)
        prop_list = IBus.PropList.new()
        prop_list.append(self._mode_main_prop)
        self._mode_main_prop_list = prop_list
        self.register_properties(prop_list)
        self._current_input_mode = normalized
        self._mode_property_registered = True

    def _update_mode_property(self, mode: str) -> None:
        normalized = self._normalize_mode(mode)
        if not self._mode_property_registered or self._mode_main_prop is None:
            self._register_mode_property(normalized)
            return

        active_label = MODE_NIDA_LABEL if normalized == "nida" else MODE_ROMAN_LABEL
        active_symbol = MODE_NIDA_SYMBOL if normalized == "nida" else MODE_ROMAN_SYMBOL

        main_prop = self._mode_main_prop
        main_prop.set_label(IBus.Text.new_from_string(active_label))
        main_prop.set_symbol(IBus.Text.new_from_string(active_symbol))
        main_prop.set_tooltip(IBus.Text.new_from_string(MODE_TOOLTIP))
        main_prop.set_state(IBus.PropState.UNCHECKED)

        roman_sub = self._mode_sub_props.get(MODE_PROPERTY_ROMAN_KEY)
        nida_sub = self._mode_sub_props.get(MODE_PROPERTY_NIDA_KEY)
        if roman_sub is not None:
            roman_sub.set_state(
                IBus.PropState.CHECKED if normalized == "roman" else IBus.PropState.UNCHECKED
            )
        if nida_sub is not None:
            nida_sub.set_state(
                IBus.PropState.CHECKED if normalized == "nida" else IBus.PropState.UNCHECKED
            )

        mode_changed = normalized != self._current_input_mode
        self._current_input_mode = normalized

        self.update_property(main_prop)
        if roman_sub is not None:
            self.update_property(roman_sub)
        if nida_sub is not None:
            self.update_property(nida_sub)

        if mode_changed and self._mode_main_prop_list is not None:
            self.register_properties(self._mode_main_prop_list)

    @staticmethod
    def _caps_lock_state() -> Optional[bool]:
        if Gdk is None:
            return None
        try:
            keymap = Gdk.Keymap.get_default()
            if keymap is None:
                return None
            return bool(keymap.get_caps_lock_state())
        except Exception as err:
            log_line(f"caps_lock_state unavailable err={err}")
            return None

    def _sync_mode_from_caps_lock_indicator(self) -> bool:
        caps_on = self._caps_lock_state()
        if caps_on is None:
            input_mode = self._pending_caps_input_mode
            if input_mode is None:
                log_line("caps_lock sync skipped: no indicator state or pending target")
                return False
            log_line(
                "caps_lock sync skipped indicator unavailable pending_input_mode=%s"
                % input_mode
            )
            return False
        input_mode = "nida" if caps_on else "roman"
        self._pending_caps_input_mode = input_mode
        response = self._bridge_call({"cmd": "set_input_mode", "input_mode": input_mode})
        log_line(
            "caps_lock sync caps_on=%s input_mode=%s"
            % (caps_on, str((response.snapshot or {}).get("input_mode", "unknown")))
        )
        return False

    def _set_input_mode_from_caps_target(self, input_mode: str, source: str) -> None:
        self._pending_caps_input_mode = input_mode
        response = self._bridge_call({"cmd": "set_input_mode", "input_mode": input_mode})
        log_line(
            "caps_lock %s target=%s snapshot_input_mode=%s"
            % (source, input_mode, str((response.snapshot or {}).get("input_mode", "unknown")))
        )

    def _handle_caps_lock_key(self, state: int) -> bool:
        self._cancel_pending_refinement()
        if int(state) & STATE_RELEASE_MASK:
            log_line(f"caps_lock release ignored state={state}")
            return False
        caps_was_on = bool(int(state) & STATE_LOCK_MASK)
        predicted_mode = "roman" if caps_was_on else "nida"
        self._set_input_mode_from_caps_target(predicted_mode, "predicted")
        GLib.timeout_add(40, self._sync_mode_from_caps_lock_indicator)
        GLib.timeout_add(160, self._sync_mode_from_caps_lock_indicator)
        return False

    def _clear_composition_ui_for_commit(self, response: BridgeResponse) -> None:
        snapshot = response.snapshot or {}
        self._last_preedit = ""
        self._last_raw_preedit = str(snapshot.get("raw_preedit", ""))
        self.update_preedit_text(IBus.Text.new_from_string(""), 0, False)
        self._table.clear()
        self.update_lookup_table(self._table, False)
        self.hide_auxiliary_text()

    def _cancel_pending_refinement(self) -> None:
        self._refinement.cancel()

    def _schedule_refinement(self, raw_preedit: str) -> None:
        self._refinement.schedule(raw_preedit)

    @staticmethod
    def _is_refinement_trigger_key(keyval: int) -> bool:
        ch = chr(keyval) if 0 <= keyval <= sys.maxunicode else ""
        return bool(ch) and ch.isascii() and ch.isprintable() and not ch.isspace()

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

    def do_process_key_event(self, keyval: int, keycode: int, state: int) -> bool:
        if int(keyval) == KEY_CAPS_LOCK:
            return self._handle_caps_lock_key(int(state))
        self._cancel_pending_refinement()
        if int(keyval) in (KEY_RETURN, KEY_KP_ENTER) and self._last_preedit:
            self._last_preedit = ""
            self.update_preedit_text(IBus.Text.new_from_string(""), 0, False)
        consumed = False
        try:
            payload = {
                "cmd": "process_key_event",
                "keyval": int(keyval),
                "keycode": int(keycode),
                "state": int(state),
            }
            response = self._call_bridge_raw(payload)
            consumed = response.consumed
            self._apply_response(response)
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
            if int(keyval) in (KEY_RETURN, KEY_KP_ENTER):
                preedit_after = self._last_preedit
                focus_delta = self._focus_events - self._last_enter_focus_events
                reset_delta = self._reset_events - self._last_enter_reset_events
                self._last_enter_focus_events = self._focus_events
                self._last_enter_reset_events = self._reset_events
                log_line(
                    "enter_path keyval=%s keycode=%s state=%s consumed=%s commit_len=%s "
                    "preedit_before=0 preedit_after=%s focus_events=%s reset_events=%s "
                    "focus_delta=%s reset_delta=%s capabilities=%s purpose=%s hints=%s "
                    "surrounding_len=%s surrounding_cursor=%s surrounding_anchor=%s"
                    % (
                        keyval,
                        keycode,
                        state,
                        response.consumed,
                        len(response.commit_text or ""),
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
            raw_preedit = str(response.snapshot.get("raw_preedit", ""))
            if (
                response.consumed
                and response.commit_text is None
                and self._is_refinement_trigger_key(int(keyval))
            ):
                self._schedule_refinement(raw_preedit)
            return response.consumed
        except Exception as err:
            log_line(f"process_key_event failed keyval={keyval} keycode={keycode} state={state} err={err}")
            return consumed

    def do_focus_in(self) -> None:
        self._focus_events += 1
        log_line(f"focus_in count={self._focus_events}")
        if self._mode_main_prop_list is not None:
            self.register_properties(self._mode_main_prop_list)
        self._bridge_call({"cmd": "focus_in"})

    def do_focus_in_id(self, object_path: str, client: str) -> None:
        log_line(f"focus_in_id object_path={object_path} client={client}")
        self.do_focus_in()

    def do_focus_out(self) -> None:
        self._cancel_pending_refinement()
        self._focus_events += 1
        log_line(f"focus_out count={self._focus_events}")
        self._bridge_call({"cmd": "focus_out"})

    def do_focus_out_id(self, object_path: str) -> None:
        log_line(f"focus_out_id object_path={object_path}")
        self.do_focus_out()

    def do_reset(self) -> None:
        self._cancel_pending_refinement()
        self._reset_events += 1
        log_line(f"reset count={self._reset_events}")
        self._bridge_call({"cmd": "reset"})

    def do_enable(self) -> None:
        log_line("enable")
        if self._mode_main_prop_list is not None:
            self.register_properties(self._mode_main_prop_list)
        self._bridge_call({"cmd": "enable"})

    def do_disable(self) -> None:
        self._cancel_pending_refinement()
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

    def do_property_activate(self, prop_name: str, prop_state: int) -> None:
        if prop_name == MODE_PROPERTY_KEY:
            log_line(f"mode property activate state={prop_state}")
            self._bridge_call({"cmd": "toggle_input_mode"})
            return
        if prop_name == MODE_PROPERTY_ROMAN_KEY:
            log_line(f"mode sub-property activate roman state={prop_state}")
            self._bridge_call({"cmd": "set_input_mode", "input_mode": "roman"})
            return
        if prop_name == MODE_PROPERTY_NIDA_KEY:
            log_line(f"mode sub-property activate nida state={prop_state}")
            self._bridge_call({"cmd": "set_input_mode", "input_mode": "nida"})
            return
        super().do_property_activate(prop_name, prop_state)

    def do_destroy(self) -> None:
        self._cancel_pending_refinement()
        with self._bridge_lock:
            self._bridge.shutdown()
        super().do_destroy()


class KhmerIMEFactory(IBus.Factory):
    def __init__(self, bus: IBus.Bus, bridge_path: Path):
        super().__init__(connection=bus.get_connection(), object_path=IBus.PATH_FACTORY)
        self._bridge_path = bridge_path
        self._engine_id = 0

    def do_create_engine(self, engine_name: str) -> IBus.Engine:
        initial_input_mode = {
            ENGINE_NAME: "roman",
            ENGINE_NIDA_NAME: "nida",
        }.get(engine_name)
        if initial_input_mode is None:
            log_line(f"create_engine unexpected engine_name={engine_name}")
            raise RuntimeError(f"unexpected engine name: {engine_name}")
        object_path = f"/org/freedesktop/IBus/KhmerIME/Engine/{self._engine_id}"
        self._engine_id += 1
        log_line(f"create_engine engine_name={engine_name} object_path={object_path} input_mode={initial_input_mode}")
        return KhmerIMEEngine(
            connection=self.get_connection(),
            object_path=object_path,
            bridge_path=self._bridge_path,
            initial_input_mode=initial_input_mode,
        )


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
    if not bus.is_connected():
        log_line("startup failed: IBus daemon is not running")
        print("error: IBus daemon is not running", file=sys.stderr)
        return 1
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
        register_component(IBus, bus, exec_path)
        log_line("registered component for non-ibus launch")

    signal.signal(signal.SIGTERM, lambda *_: loop.quit())
    signal.signal(signal.SIGINT, lambda *_: loop.quit())
    loop.run()
    factory.destroy()
    return 0


if __name__ == "__main__":
    sys.exit(main())
