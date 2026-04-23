# khmerime_linux_ibus

Linux-first adapter package.

## Owns
- `khmerime_ibus_bridge` Rust JSON-line bridge binary
- Desktop history file persistence under `~/.config/khmerime/`
- Linux adapter protocol types consumed by the Python engine

## Contract
- Snapshot shape must remain compatible with `scripts/khmerime_ibus_engine.py`
