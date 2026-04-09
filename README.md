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

Operational guide:

```
docs/development.md
```

Run the web app:

```
$ make web
```

Run the web app on a fixed phone-accessible address:

```
$ make web-phone
```

Default address:

```
http://<your-lan-ip>:4173
```

Override the host or port if needed:

```
$ ADDR=0.0.0.0 PORT=8080 bash scripts/serve_web_phone.sh
```

Run the desktop app:

```
$ make desktop
```

Run the CLI and print the number of entries:

```
$ make stats
```

Run the CLI and print the top suggestions for a roman query:

```
$ make suggest QUERY=tver
```

Use a different lexicon file:

```
$ cargo run --bin lookup_cli -- --data data/your_lexicon.tsv suggest tver
```

Run the full Rust test suite:

```
$ make test
```

Run the WFST golden snapshot test:

```
$ make test-golden
```

## Behavior

- Loads the lexicon from a TSV file or from the embedded default dataset
- Builds a fuzzy n-gram search index
- Reranks candidates with edit-distance similarity
- Reorders candidates using learned history
- The Dioxus app supports `Space` to cycle, `1`-`5` to choose the visible suggestion, `Enter` or `Shift+Space` to commit, `Tab` to step through suggestions, and `Alt+Ctrl+K` to toggle conversion
- Web persists editor state and learned history in browser storage; desktop currently starts with a fresh session each launch
