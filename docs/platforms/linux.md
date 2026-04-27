# Linux (IBus)

Linux native input currently uses Ubuntu/GNOME-style IBus source switching.

For the shared platform workflow, read [`docs/platforms/README.md`](README.md).

For the detailed explanation of how the keyboard path works, read:

- [`docs/ubuntu_ibus.md`](../ubuntu_ibus.md)

## Runtime Pieces

- `adapters/linux-ibus`: Rust bridge + desktop history persistence
- `adapters/linux-ibus/python/khmerime_ibus_engine.py`: IBus callback adapter
- `crates/session/src/ime_session.rs`: platform-neutral IME session state and key behavior
- `scripts/platforms/linux/ibus/install_engine.sh`: developer-local IBus install script

## Common Commands

Developer-local install:

```bash
make ibus-install
make ibus-smoke
make ibus-uninstall
```

Build an installable Debian package:

```bash
make linux-package
```

The package is written to:

```text
dist/linux/khmerime_<version>_amd64.deb
```

Inspect the package:

```bash
dpkg-deb -I dist/linux/khmerime_0.1.0_amd64.deb
dpkg-deb -c dist/linux/khmerime_0.1.0_amd64.deb
```

Install on Ubuntu/Debian:

```bash
sudo apt install ./dist/linux/khmerime_0.1.0_amd64.deb
```

After install, restart IBus or log out and back in:

```bash
ibus restart
```

Then add KhmerIME from:

```text
Settings -> Keyboard -> Input Sources -> Khmer -> KhmerIME
```

Remove the package:

```bash
sudo apt remove khmerime
```

## Package Contents

The `.deb` installs:

```text
/usr/libexec/khmerime/khmerime-ibus-engine
/usr/libexec/khmerime/ibus_segment_preview.py
/usr/libexec/khmerime/khmerime-ibus-bridge
/usr/share/ibus/component/khmerime.xml
```

The package maintainer scripts refresh the IBus cache with `ibus write-cache || true`.
They print restart guidance but do not forcibly restart the user's input session.
