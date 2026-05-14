"""IBus component metadata helpers for KhmerIME."""

from __future__ import annotations

from pathlib import Path
from typing import Any

SERVICE_NAME = "org.freedesktop.IBus.KhmerIME"
ENGINE_NAME = "khmerime"
ENGINE_NIDA_NAME = "khmerime-nida"
ENGINE_LONGNAME = "KhmerIME"
ENGINE_NIDA_LONGNAME = "KhmerIME NIDA"
ENGINE_DESCRIPTION = "Khmer romanization IME powered by KhmerIME"
ENGINE_NIDA_DESCRIPTION = "KhmerIME direct NIDA input mode"
ENGINE_LANGUAGE = "km"
ENGINE_LAYOUT = "us"
ENGINE_SYMBOL = "ខ"


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
            <icon_prop_key>InputMode</icon_prop_key>
            <symbol>{ENGINE_SYMBOL}</symbol>
        </engine>
        <engine>
            <name>{ENGINE_NIDA_NAME}</name>
            <longname>{ENGINE_NIDA_LONGNAME}</longname>
            <description>{ENGINE_NIDA_DESCRIPTION}</description>
            <language>{ENGINE_LANGUAGE}</language>
            <license>MIT</license>
            <author>KhmerIME contributors</author>
            <icon></icon>
            <layout>{ENGINE_LAYOUT}</layout>
            <icon_prop_key>InputMode</icon_prop_key>
            <symbol>{ENGINE_SYMBOL}</symbol>
        </engine>
    </engines>
</component>"""


def register_component(ibus: Any, bus: Any, exec_path: Path) -> None:
    component = ibus.Component.new(
        SERVICE_NAME,
        "KhmerIME input method engine",
        "0.1.0",
        "MIT",
        "KhmerIME contributors",
        "https://github.com/darhnoel/khmerime",
        str(exec_path),
        "khmerime",
    )
    for name, longname, description in (
        (ENGINE_NAME, ENGINE_LONGNAME, ENGINE_DESCRIPTION),
        (ENGINE_NIDA_NAME, ENGINE_NIDA_LONGNAME, ENGINE_NIDA_DESCRIPTION),
    ):
        engine = ibus.EngineDesc.new(
            name,
            longname,
            description,
            ENGINE_LANGUAGE,
            "MIT",
            "KhmerIME contributors",
            "",
            ENGINE_LAYOUT,
        )
        component.add_engine(engine)
    bus.register_component(component)
