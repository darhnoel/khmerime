# Linux (IBus)

Linux native input currently uses Ubuntu/GNOME-style IBus source switching.

For the detailed explanation of how the keyboard path works, read:

- [`docs/ubuntu_ibus.md`](../ubuntu_ibus.md)

## Runtime Pieces

- `adapters/linux-ibus`: Rust bridge + desktop history persistence
- `adapters/linux-ibus/python/khmerime_ibus_engine.py`: IBus callback adapter
- `crates/session/src/ime_session.rs`: platform-neutral IME session state and key behavior
- `scripts/platforms/linux/ibus/install_engine.sh`: developer-local IBus install script

## Common Commands

```bash
make ibus-install
make ibus-smoke
make ibus-uninstall
```
