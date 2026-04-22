# Roman Lookup

Standalone Rust lookup engine with:
- a separate CSV lexicon such as `data/your_lexicon.csv`
- a Dioxus MVP that runs on web and desktop

## Layout

- Core logic: [src/roman_lookup.rs](src/roman_lookup.rs)
- Dioxus app: [src/main.rs](src/main.rs)
- CLI: [src/bin/lookup_cli.rs](src/bin/lookup_cli.rs)
- Architecture guide: [docs/architecture.md](docs/architecture.md)
- Default embedded data: `data/roman_lookup.csv`

## Usage

Preferred entrypoint:

```
$ make help
```

### Web Release Base Path

For subpath hosting (for example `/khmerime-beta/`), build with one variable:

```bash
WEB_BASE_PATH=khmerime-beta make web-release
```

The release script will:
- pass `--base-path` to Dioxus build output, and
- export the same value as `KHMERIME_BASE_PATH` for runtime `.bin` fetch URLs.

### Startup Diagnostics (iOS Critical Path)

On `wasm32` + `fetch-data`, the app supports runtime startup profiles via query string:

- `?startup_profile=defer_full` (default): phase-A legacy ready first, then background full promotion.
- `?startup_profile=baseline`: wait for full engine before ready.
- `?startup_profile=lexicon_only`: lexicon-only phase (skip khPOS fetch/promotion).
- `?startup_profile=baseline_compression_audit`: baseline plus response header capture for compression/cache.

Each startup run emits a single structured console report with per-stage timestamps.

### khPOS Size Trimming Knobs

Build-time optional knobs for `khpos.stats.bin` experimentation:

- `KHPOS_SURFACE_MIN_COUNT=<u32>`: drop low-frequency `surface_unigrams`.
- `KHPOS_SURFACE_TOP_N=<usize>`: keep only top-N `surface_unigrams` by count.

Example:

```bash
KHPOS_SURFACE_MIN_COUNT=2 KHPOS_SURFACE_TOP_N=120000 make web-release
```
