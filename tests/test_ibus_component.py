from pathlib import Path
import sys


PYTHON_ADAPTER = Path(__file__).resolve().parents[1] / "adapters" / "linux-ibus" / "python"
sys.path.insert(0, str(PYTHON_ADAPTER))

from ibus_component import component_xml  # noqa: E402


def test_component_xml_registers_roman_and_nida_engines() -> None:
    xml = component_xml(Path("/usr/libexec/khmerime/khmerime-ibus-engine"))

    assert "<name>khmerime</name>" in xml
    assert "<longname>KhmerIME</longname>" in xml
    assert "<name>khmerime-nida</name>" in xml
    assert "<longname>KhmerIME NIDA</longname>" in xml
    assert xml.count("<icon_prop_key>InputMode</icon_prop_key>") == 2
