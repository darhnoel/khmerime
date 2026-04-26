# Development Workflow

This document is the operational entrypoint for working on `khmerime`.

If you are not sure how to run something, start with:

```bash
make help
```

## Start Here

Read these in order when you need context:

1. `README.md` for the product summary
2. `docs/development.md` for how to run and verify the project
3. `docs/architecture.md` for the decoder and UI architecture
4. `docs/khpos_mechanism.md` for the `khPOS` integration design and status
5. `docs/ubuntu_ibus.md` when working on native Ubuntu input-source behavior

## Common Commands

Use the root `Makefile` for common tasks:

```bash
make web
make web-phone
make desktop
make stats
make suggest QUERY=tver
make suggest-wfst QUERY=tver
make suggest-shadow QUERY=tver
make shadow-eval QUERIES=path/to/queries.txt
make test
make test-golden
make test-ui
make ibus-install
make ibus-uninstall
make ibus-smoke
make fmt
make paper-current
make paper-current-clean
```

## What Each Command Does

```text
make web
  Run the Dioxus web app locally.

make web-phone
  Run the web app on 0.0.0.0:4173 through scripts/web/serve_phone.sh.
  Override with ADDR=... and PORT=....

make desktop
  Run the Dioxus desktop app.

make stats
  Print the number of embedded lexicon entries.

make suggest QUERY=tver
  Print legacy-mode suggestions for one roman query.

make suggest-wfst QUERY=tver
  Print WFST-mode suggestions for one roman query.

make suggest-shadow QUERY=tver
  Print shadow-mode suggestions for one roman query.

make shadow-eval QUERIES=path/to/queries.txt [MODE=shadow|wfst|hybrid] [OUTPUT=report.txt]
  Run decoder comparison on a query file.

make test
  Run the Rust test suite.

make test-golden
  Run the WFST golden snapshot test only.

make test-ui
  Run the browser/UI Python test file.

make ibus-install
  Build and install KhmerIME IBus engine files for Ubuntu desktop testing.
  On Ubuntu GNOME, this may prompt for sudo because IBus component discovery
  is system-path based (`/usr/share/ibus/component`).

make ibus-uninstall
  Remove KhmerIME IBus engine files.

make ibus-smoke
  Run bridge protocol smoke checks and IBus engine discovery checks.

make fmt
  Run cargo fmt --all.

make paper-current
  Build papers/current-implementation/khmerime_current_implementation_paper.pdf.

make paper-current-clean
  Remove LaTeX build byproducts from papers/current-implementation.
```

## Defaults And Rules

- Prefer `make` targets over retyping raw commands.
- Run `make fmt` after changing Rust source files.
- Use `make suggest`, `make suggest-wfst`, or `make shadow-eval` to inspect decoder behavior before editing tests or snapshots.
- Keep the current paper source and generated PDF under `papers/current-implementation/`.
- Keep `data/roman_lookup.csv` as `roman,target`.
- Treat this repository as a Khmer IME, not just a dictionary lookup project.
- For Ubuntu native switching work, keep system source switching (`Super+Space`)
  as the primary on/off path in v1.

## Raw Commands

If a `make` target is not enough, these are the underlying commands:

```bash
cd apps/dioxus-app && dx serve
cd apps/dioxus-app && dx serve --platform desktop
bash scripts/web/serve_phone.sh
cargo run -p khmerime_lookup_cli --bin lookup_cli -- stats
cargo run -p khmerime_lookup_cli --bin lookup_cli -- suggest tver
cargo run -p khmerime_lookup_cli --bin lookup_cli -- --decoder-mode wfst suggest tver
cargo run -p khmerime_lookup_cli --bin lookup_cli -- --decoder-mode shadow shadow-eval path/to/queries.txt
cargo test
cargo test --test decoder_golden
python3 -m pytest tests/test_web_ui.py
bash scripts/platforms/linux/ibus/install_engine.sh
bash scripts/platforms/linux/ibus/uninstall_engine.sh
bash scripts/platforms/linux/ibus/smoke_test.sh
cd papers/current-implementation && TEXMFVAR=/tmp/texmf-var lualatex -interaction=nonstopmode -halt-on-error khmerime_current_implementation_paper.tex
```

## For Codex

Future Codex runs should:

1. read `docs/development.md`
2. use `make help` when looking for run commands
3. prefer `make` targets unless a lower-level command is necessary
4. keep paper work in `papers/current-implementation/`
