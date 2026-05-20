# Cache SharedTransliteratorData on disk between bridge restarts

The IBus engine subprocess is torn down on every Super+Space switch-away, and recreated on switch-back. Each cold start rebuilds the **SharedTransliteratorData** from scratch — empirically ~830 ms (ngram) to ~1280 ms (SymSpell) on a release build, with ~95% of the time spent on in-memory index construction (not disk parsing). We will serialize the built `SharedTransliteratorData` with bincode to `~/.cache/khmerime/shared_data.<key>.bin`, where `<key>` is a blake3 hash of (schema version, bridge binary mtime, sorted input-file mtimes, search-index backend selector). On startup the bridge tries to deserialize the cache; a hit skips the build entirely, a miss rebuilds and atomically rewrites the file.

We chose this over keeping the bridge subprocess alive across IBus engine teardown (P1) because we are in an experimental phase with frequent recompile-and-reinstall cycles — a long-lived daemon would silently serve stale code after every reinstall. The disk cache invalidates automatically on binary mtime change, costing one slow rebuild per `cargo install` and serving fast deserialization for every subsequent start. We chose bincode over rkyv because the dominant saved cost is index construction (~750 ms), not deserialization (~200 ms estimated for this dataset); rkyv's zero-copy advantage doesn't justify its larger code change and new failure modes.

## Considered Options

- **Keep bridge subprocess alive across engine teardown (P1)** — would eliminate switch-back latency entirely but holds stale binaries across reinstalls during dev; revisit when the project stabilizes.
- **Rebuild faster instead of caching** — investigated, but the bulk of the cost is irreducible index construction over the full lexicon.
- **rkyv zero-copy serialization** — deferred. If bincode deserialize is still slow enough to matter after this lands, reconsider.

## Consequences

- A cache miss writes ~tens of MB to `~/.cache/khmerime/`; a daily GC of `shared_data.*.bin` files older than 7 days runs on each successful write to bound disk use.
- Corrupt or unreadable cache files are treated as misses with a logged warning; write failures fall through to no-caching without failing startup.
- The cache key is computed in the bridge binary, so any change to the cache-key schema requires bumping the schema-version constant.
