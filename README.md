# Roman Lookup

Standalone Rust lookup engine with:
- a separate TSV lexicon such as `data/your_lexicon.tsv`
- a Dioxus MVP that runs on web and desktop

## Layout

- Core logic: [src/roman_lookup.rs](src/roman_lookup.rs)
- Dioxus app: [src/main.rs](src/main.rs)
- CLI: [src/bin/lookup_cli.rs](src/bin/lookup_cli.rs)
- Architecture guide: [docs/architecture.md](docs/architecture.md)
- Default embedded data: `data/roman_lookup.tsv`

## Usage

Preferred entrypoint:

```
$ make help
```