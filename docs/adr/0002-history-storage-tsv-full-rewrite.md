# Keep history as TSV with full-file rewrite on every commit

User history (learned unigram counts) lives in a flat `history.tsv` file at `~/.config/khmerime/history.tsv` (Linux) or `%APPDATA%\khmerime\history.tsv` (Windows). Every commit that changes history rewrites the whole file. We considered SQLite and binary formats (bincode, rkyv) and rejected them because the file plateaus at the size of the user's Khmer vocabulary (~25-30K entries, ~600 KB), full-rewrite latency stays under ~5 ms on SSD, and reads never touch disk during typing (the file is loaded once into a `HashMap` at session start). The pollution vector that made history grow combinatorially — multi-segment commits learning the entire concatenated phrase as one key — is being fixed in the same change that produced this ADR, so the natural ceiling now actually holds.

TSV also keeps the file human-readable: `grep`, `cat`, and manual editing work, which has been useful during debugging. Binary formats would save ~4 ms at startup and ~50 KB on disk in exchange for losing all of that.

## When to revisit

This decision should be reconsidered when **any** of the following is true:

- Bigrams land. Bigram tables are O(N²) in vocabulary and can easily reach 100K+ entries (~10-15 MB), at which point full-file rewrite latency (~150-300 ms) becomes felt during Space.
- Multiple processes need to write the same history file concurrently (TSV has no locking; current adapters are single-writer).
- Write latency exceeds ~10 ms in profiling under realistic vocabulary.

The `HistoryStore` trait in [crates/session/src/ime_session.rs](../../crates/session/src/ime_session.rs) is the single seam to swap, so the cost of deferring is low.
