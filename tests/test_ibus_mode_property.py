"""Unit tests for the IBus mode property symbol switching.

These tests stub `gi.repository.IBus`, `GLib`, and `Gdk` so the engine module
loads without a running IBus daemon. They verify that switching between Roman
and NIDA mutates the *same* `IBus.Property` instance, sets the expected symbol
('R' or 'ខ'), and triggers `update_property` -- which is what GNOME Shell's
input-source indicator subscribes to.
"""

from __future__ import annotations

import sys
import types
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional


PYTHON_ADAPTER = Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"
sys.path.insert(0, str(PYTHON_ADAPTER))


# ---- Stub IBus / GLib / Gdk ------------------------------------------------


class _StubText:
    def __init__(self, text: str) -> None:
        self.text = text

    @staticmethod
    def new_from_string(text: str) -> "_StubText":
        return _StubText(text)

    def get_text(self) -> str:
        return self.text


class _StubAttrList:
    def __init__(self) -> None:
        self.attrs: List[Any] = []

    @staticmethod
    def new() -> "_StubAttrList":
        return _StubAttrList()

    def append(self, attr: Any) -> None:
        self.attrs.append(attr)


@dataclass
class _StubProperty:
    key: str
    prop_type: int
    label: Optional[_StubText] = None
    icon: str = ""
    tooltip: Optional[_StubText] = None
    sensitive: bool = True
    visible: bool = True
    state: int = 0
    sub_props: Optional["_StubPropList"] = None
    symbol: Optional[_StubText] = None
    update_count: int = 0

    @staticmethod
    def new(
        key: str,
        prop_type: int,
        label: _StubText,
        icon: str,
        tooltip: _StubText,
        sensitive: bool,
        visible: bool,
        state: int,
        sub_props: Optional["_StubPropList"],
    ) -> "_StubProperty":
        return _StubProperty(
            key=key,
            prop_type=prop_type,
            label=label,
            icon=icon,
            tooltip=tooltip,
            sensitive=sensitive,
            visible=visible,
            state=state,
            sub_props=sub_props,
        )

    def set_symbol(self, symbol: _StubText) -> None:
        self.symbol = symbol

    def set_label(self, label: _StubText) -> None:
        self.label = label

    def set_tooltip(self, tooltip: _StubText) -> None:
        self.tooltip = tooltip

    def set_state(self, state: int) -> None:
        self.state = state

    def set_visible(self, visible: bool) -> None:
        self.visible = visible

    def set_sensitive(self, sensitive: bool) -> None:
        self.sensitive = sensitive

    def get_key(self) -> str:
        return self.key


class _StubPropList:
    def __init__(self) -> None:
        self.props: List[_StubProperty] = []

    @staticmethod
    def new() -> "_StubPropList":
        return _StubPropList()

    def append(self, prop: _StubProperty) -> None:
        self.props.append(prop)


class _StubLookupTable:
    def __init__(self) -> None:
        self.cleared = 0
        self.candidates: List[Any] = []
        self.cursor = 0

    @staticmethod
    def new(_size: int, _cursor: int, _round: bool, _orient: bool) -> "_StubLookupTable":
        return _StubLookupTable()

    def clear(self) -> None:
        self.cleared += 1
        self.candidates = []

    def append_candidate(self, candidate: Any) -> None:
        self.candidates.append(candidate)

    def set_cursor_pos(self, pos: int) -> None:
        self.cursor = pos


class _StubPropType:
    NORMAL = 0
    TOGGLE = 1
    RADIO = 2
    MENU = 3
    SEPARATOR = 4


class _StubPropState:
    UNCHECKED = 0
    CHECKED = 1
    INCONSISTENT = 2


class _StubModifierType:
    LOCK_MASK = 1 << 1
    RELEASE_MASK = 1 << 30


class _StubAttrUnderline:
    SINGLE = 1


class _StubFactoryBase:
    """Stand-in for IBus.Factory; not exercised but referenced at module import."""

    def __init__(self, *_args: Any, **_kwargs: Any) -> None:  # pragma: no cover
        pass


class _StubBus:
    def get_connection(self) -> None:  # pragma: no cover
        return None


class _StubEngineBase:
    """Stand-in for IBus.Engine; constructor is a no-op."""

    def __init__(self, *_args: Any, **_kwargs: Any) -> None:
        self.registered_prop_lists: List[_StubPropList] = []
        self.updated_props: List[_StubProperty] = []
        self.committed_text: List[str] = []
        self.preedit_updates: List[Any] = []
        self.lookup_updates: List[Any] = []
        self.aux_updates: List[Any] = []

    def register_properties(self, prop_list: _StubPropList) -> None:
        self.registered_prop_lists.append(prop_list)

    def update_property(self, prop: _StubProperty) -> None:
        prop.update_count += 1
        self.updated_props.append(prop)

    def commit_text(self, text: _StubText) -> None:
        self.committed_text.append(text.get_text())

    def update_preedit_text(self, text: _StubText, cursor: int, visible: bool) -> None:
        self.preedit_updates.append((text.get_text(), cursor, visible))

    def update_lookup_table(self, table: _StubLookupTable, visible: bool) -> None:
        self.lookup_updates.append((len(table.candidates), visible))

    def update_auxiliary_text(self, text: _StubText, visible: bool) -> None:
        self.aux_updates.append((text.get_text(), visible))

    def show_auxiliary_text(self) -> None:  # pragma: no cover - trivial
        pass

    def hide_auxiliary_text(self) -> None:  # pragma: no cover - trivial
        pass


def attr_underline_new(*_args: Any, **_kwargs: Any) -> Any:
    return ("underline",) + tuple(_args)


class _StubIBus(types.SimpleNamespace):
    pass


def _install_stub_modules() -> None:
    ibus = _StubIBus()
    ibus.Engine = _StubEngineBase
    ibus.Factory = _StubFactoryBase
    ibus.Bus = _StubBus
    ibus.Text = _StubText
    ibus.AttrList = _StubAttrList
    ibus.attr_underline_new = attr_underline_new
    ibus.AttrUnderline = _StubAttrUnderline
    ibus.Property = _StubProperty
    ibus.PropList = _StubPropList
    ibus.LookupTable = _StubLookupTable
    ibus.PropType = _StubPropType
    ibus.PropState = _StubPropState
    ibus.ModifierType = _StubModifierType
    ibus.PATH_FACTORY = "/org/freedesktop/IBus/Factory"

    def _init() -> None:  # pragma: no cover - trivial
        return None

    ibus.init = _init

    glib = types.SimpleNamespace(
        timeout_add=lambda *_a, **_k: 0,
        source_remove=lambda *_a, **_k: False,
        idle_add=lambda *_a, **_k: 0,
        MainLoop=lambda: types.SimpleNamespace(run=lambda: None, quit=lambda: None),
    )

    gdk = types.SimpleNamespace(Keymap=types.SimpleNamespace(get_default=lambda: None))

    gi_module = types.ModuleType("gi")
    gi_module.require_version = lambda *_a, **_k: None
    repository = types.ModuleType("gi.repository")
    repository.IBus = ibus
    repository.GLib = glib
    repository.Gdk = gdk
    gi_module.repository = repository

    sys.modules["gi"] = gi_module
    sys.modules["gi.repository"] = repository
    sys.modules["gi.repository.IBus"] = ibus  # type: ignore[assignment]
    sys.modules["gi.repository.GLib"] = glib  # type: ignore[assignment]
    sys.modules["gi.repository.Gdk"] = gdk  # type: ignore[assignment]


_install_stub_modules()


# ---- Stub the bridge client so engine construction doesn't spawn a process --


@dataclass
class _FakeBridgeResponse:
    ok: bool = True
    consumed: bool = False
    commit_text: Optional[str] = None
    snapshot: Dict[str, Any] = field(default_factory=dict)
    readiness: str = "full"
    error: Optional[str] = None


class _FakeBridgeClient:
    def __init__(
        self,
        _path: Path,
        *,
        initial_input_mode: str = "roman",
        deferred_segmented_preview: bool = False,
    ) -> None:
        self.initial_input_mode = initial_input_mode
        self.deferred_segmented_preview = deferred_segmented_preview
        self.calls: List[Dict[str, Any]] = []
        self._current_mode = initial_input_mode

    def call(self, payload: Dict[str, Any]) -> _FakeBridgeResponse:
        self.calls.append(payload)
        cmd = payload.get("cmd")
        if cmd == "set_input_mode":
            self._current_mode = str(payload.get("input_mode", self._current_mode))
        elif cmd == "toggle_input_mode":
            self._current_mode = "nida" if self._current_mode == "roman" else "roman"
        return _FakeBridgeResponse(snapshot={"input_mode": self._current_mode})

    def shutdown(self) -> None:  # pragma: no cover - trivial
        pass


import ibus_bridge_client  # noqa: E402

ibus_bridge_client.BridgeClient = _FakeBridgeClient  # type: ignore[misc]
ibus_bridge_client.BridgeResponse = _FakeBridgeResponse  # type: ignore[misc]

import khmerime_ibus_engine as engine_mod  # noqa: E402

engine_mod.BridgeClient = _FakeBridgeClient  # type: ignore[attr-defined]
engine_mod.BridgeResponse = _FakeBridgeResponse  # type: ignore[attr-defined]


# ---- Helpers ----------------------------------------------------------------


def _make_engine(initial_mode: str = "roman") -> engine_mod.KhmerIMEEngine:
    return engine_mod.KhmerIMEEngine(
        connection=None,
        object_path="/test/engine",
        bridge_path=Path("/nonexistent/bridge"),
        initial_input_mode=initial_mode,
    )


# ---- Tests ------------------------------------------------------------------


def test_initial_symbol_is_R_in_roman_mode() -> None:
    eng = _make_engine("roman")

    prop = eng._mode_main_prop
    assert prop is not None
    assert prop.key == engine_mod.MODE_PROPERTY_KEY
    assert prop.prop_type == _StubPropType.MENU
    assert prop.symbol is not None
    assert prop.symbol.get_text() == "R"
    assert prop.label.get_text() == "Roman"
    # property list was registered on construction
    assert len(eng.registered_prop_lists) >= 1
    # the registered list contains exactly the main InputMode prop
    assert eng.registered_prop_lists[-1].props[-1] is prop


def test_initial_symbol_is_khmer_in_nida_mode() -> None:
    eng = _make_engine("nida")

    prop = eng._mode_main_prop
    assert prop is not None
    assert prop.symbol is not None
    assert prop.symbol.get_text() == "ខ"
    assert prop.label.get_text() == "NIDA"


def test_switch_to_nida_sets_symbol_kh_and_reuses_property_object() -> None:
    eng = _make_engine("roman")
    original_prop = eng._mode_main_prop
    assert original_prop is not None

    eng._update_mode_property("nida")

    # SAME instance must be reused so panels relying on identity see updates.
    assert eng._mode_main_prop is original_prop
    assert original_prop.symbol.get_text() == "ខ"
    assert original_prop.label.get_text() == "NIDA"
    # update_property was called on the same prop
    assert original_prop in eng.updated_props
    assert original_prop.update_count >= 1


def test_switch_back_to_roman_sets_symbol_R_and_reuses_property_object() -> None:
    eng = _make_engine("nida")
    original_prop = eng._mode_main_prop
    assert original_prop is not None

    eng._update_mode_property("roman")

    assert eng._mode_main_prop is original_prop
    assert original_prop.symbol.get_text() == "R"
    assert original_prop.label.get_text() == "Roman"
    assert original_prop.update_count >= 1


def test_mode_property_is_menu_with_roman_and_nida_subprops() -> None:
    eng = _make_engine("roman")
    prop = eng._mode_main_prop
    assert prop is not None
    assert prop.prop_type == _StubPropType.MENU

    sub_list = prop.sub_props
    assert sub_list is not None
    keys = [p.key for p in sub_list.props]
    assert engine_mod.MODE_PROPERTY_ROMAN_KEY in keys
    assert engine_mod.MODE_PROPERTY_NIDA_KEY in keys

    roman_sub = next(p for p in sub_list.props if p.key == engine_mod.MODE_PROPERTY_ROMAN_KEY)
    nida_sub = next(p for p in sub_list.props if p.key == engine_mod.MODE_PROPERTY_NIDA_KEY)
    assert roman_sub.prop_type == _StubPropType.RADIO
    assert nida_sub.prop_type == _StubPropType.RADIO
    assert roman_sub.symbol.get_text() == "R"
    assert nida_sub.symbol.get_text() == "ខ"


def test_subprop_radio_state_tracks_current_mode() -> None:
    eng = _make_engine("roman")
    roman_sub = eng._mode_sub_props[engine_mod.MODE_PROPERTY_ROMAN_KEY]
    nida_sub = eng._mode_sub_props[engine_mod.MODE_PROPERTY_NIDA_KEY]

    assert roman_sub.state == _StubPropState.CHECKED
    assert nida_sub.state == _StubPropState.UNCHECKED

    eng._update_mode_property("nida")

    assert roman_sub.state == _StubPropState.UNCHECKED
    assert nida_sub.state == _StubPropState.CHECKED


def test_update_to_same_mode_is_idempotent() -> None:
    eng = _make_engine("roman")
    prop = eng._mode_main_prop
    assert prop is not None
    before_updates = prop.update_count
    before_registers = len(eng.registered_prop_lists)

    eng._update_mode_property("roman")

    # update_property is still called (cheap, keeps panel in sync),
    # but no extra register_properties because the mode didn't change.
    assert prop.symbol.get_text() == "R"
    assert prop.update_count > before_updates
    assert len(eng.registered_prop_lists) == before_registers


def test_mode_change_re_registers_property_list() -> None:
    """A mode change should re-register the prop list so late-binding panels resync."""
    eng = _make_engine("roman")
    before = len(eng.registered_prop_lists)

    eng._update_mode_property("nida")

    assert len(eng.registered_prop_lists) == before + 1
    assert eng.registered_prop_lists[-1] is eng._mode_main_prop_list


def test_property_activate_for_subprops_calls_bridge_with_input_mode() -> None:
    eng = _make_engine("roman")
    bridge: _FakeBridgeClient = eng._bridge  # type: ignore[assignment]
    bridge.calls.clear()

    eng.do_property_activate(engine_mod.MODE_PROPERTY_NIDA_KEY, _StubPropState.CHECKED)
    assert any(c.get("cmd") == "set_input_mode" and c.get("input_mode") == "nida" for c in bridge.calls)

    eng.do_property_activate(engine_mod.MODE_PROPERTY_ROMAN_KEY, _StubPropState.CHECKED)
    assert any(c.get("cmd") == "set_input_mode" and c.get("input_mode") == "roman" for c in bridge.calls)


def test_property_activate_main_key_toggles_input_mode() -> None:
    eng = _make_engine("roman")
    bridge: _FakeBridgeClient = eng._bridge  # type: ignore[assignment]
    bridge.calls.clear()

    eng.do_property_activate(engine_mod.MODE_PROPERTY_KEY, _StubPropState.UNCHECKED)

    assert any(c.get("cmd") == "toggle_input_mode" for c in bridge.calls)


def test_focus_in_re_registers_properties() -> None:
    """gnome-shell may connect after engine init; focus_in must resync the panel."""
    eng = _make_engine("roman")
    before = len(eng.registered_prop_lists)

    eng.do_focus_in()

    assert len(eng.registered_prop_lists) == before + 1
    assert eng.registered_prop_lists[-1] is eng._mode_main_prop_list


def test_enable_re_registers_properties() -> None:
    eng = _make_engine("roman")
    before = len(eng.registered_prop_lists)

    eng.do_enable()

    assert len(eng.registered_prop_lists) == before + 1
    assert eng.registered_prop_lists[-1] is eng._mode_main_prop_list


def test_apply_snapshot_updates_symbol_to_match_input_mode() -> None:
    """`_apply_snapshot` is how the bridge informs the panel of mode changes."""
    eng = _make_engine("roman")
    prop = eng._mode_main_prop
    assert prop is not None
    assert prop.symbol.get_text() == "R"

    eng._apply_snapshot(_FakeBridgeResponse(snapshot={"input_mode": "nida"}))
    assert prop.symbol.get_text() == "ខ"

    eng._apply_snapshot(_FakeBridgeResponse(snapshot={"input_mode": "roman"}))
    assert prop.symbol.get_text() == "R"


def test_key_release_does_not_cancel_pending_refinement() -> None:
    eng = _make_engine("roman")
    cancelled = 0

    def cancel() -> None:
        nonlocal cancelled
        cancelled += 1

    def fail_call(_payload: Dict[str, Any]) -> _FakeBridgeResponse:
        raise AssertionError("release events should not call the bridge")

    eng._cancel_pending_refinement = cancel  # type: ignore[method-assign]
    eng._call_bridge_raw = fail_call  # type: ignore[method-assign]

    consumed = eng.do_process_key_event(ord("a"), 30, engine_mod.STATE_RELEASE_MASK)

    assert consumed is False
    assert cancelled == 0


def test_enter_does_not_clear_preedit_before_bridge_commit() -> None:
    eng = _make_engine("roman")
    eng._last_preedit = "nihjeasnadaiborkbrae"
    eng._last_raw_preedit = "nihjeasnadaiborkbrae"
    eng.preedit_updates.clear()

    def commit_response(_payload: Dict[str, Any]) -> _FakeBridgeResponse:
        assert eng.preedit_updates == []
        return _FakeBridgeResponse(
            consumed=True,
            commit_text="នេះជាស្នាដៃបកប្រែ",
            snapshot={"input_mode": "roman", "raw_preedit": "", "preedit": ""},
        )

    eng._call_bridge_raw = commit_response  # type: ignore[method-assign]

    consumed = eng.do_process_key_event(engine_mod.KEY_RETURN, 28, 0)

    assert consumed is True
    assert eng.committed_text == ["នេះជាស្នាដៃបកប្រែ"]
    assert eng.preedit_updates[-1] == ("", 0, False)


def test_enter_empty_commit_preserves_active_preedit() -> None:
    eng = _make_engine("roman")
    eng._last_preedit = "nihjeasnadaiborkbrae"
    eng._last_raw_preedit = "nihjeasnadaiborkbrae"
    eng.preedit_updates.clear()

    def empty_commit_response(_payload: Dict[str, Any]) -> _FakeBridgeResponse:
        return _FakeBridgeResponse(
            consumed=True,
            commit_text=None,
            snapshot={"input_mode": "roman", "raw_preedit": "", "preedit": ""},
        )

    eng._call_bridge_raw = empty_commit_response  # type: ignore[method-assign]

    consumed = eng.do_process_key_event(engine_mod.KEY_RETURN, 28, 0)

    assert consumed is True
    assert eng.committed_text == []
    assert eng.preedit_updates == []
    assert eng._last_preedit == "nihjeasnadaiborkbrae"
    assert eng._last_raw_preedit == "nihjeasnadaiborkbrae"
