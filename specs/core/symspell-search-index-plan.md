# SymSpell Search Index Plan

## Purpose

Evaluate replacing the legacy fuzzy `SearchIndex` with a SymSpell-style
delete-key index for roman lookup suggestions.

The immediate motivation is Linux IBus full-warmup latency. After sharing full
engine data, the remaining release warmup cost is mostly:

- weighted-span `RankedLexicon` index construction;
- legacy fuzzy `SearchIndex` construction.

SymSpell is a candidate for the legacy fuzzy `SearchIndex` only. It should not
replace weighted-span ranked retrieval in the same slice.

## Current Boundary

Owned module:

- `crates/core/src/roman_lookup/search_index.rs`

Current consumers:

- `crates/core/src/roman_lookup/legacy_data.rs`
- `LegacyData::suggest(...)`

Do not change in this experiment:

- `crates/core/src/roman_lookup/ranked_lexicon.rs`
- `crates/core/src/decoder/weighted_span.rs`
- decoder mode semantics
- IBus bridge/session behavior

## Hypothesis

A SymSpell-style delete-key index will reduce fuzzy index build and query cost
because it retrieves candidates from precomputed deletion neighborhoods instead
of building n-gram vectors and reranking broad matches.

Success means:

- `full_shared_data.build_legacy_data.search_index` drops meaningfully from the
  current release range of roughly `200-300 ms`;
- total release `full_warmup.end` moves closer to or below `1 s`;
- existing suggestion quality is preserved or accepted golden diffs are small
  and intentional.

## Proposed Design

Add an internal `SymSpellIndex` with:

- `exact: HashMap<String, String>`
  Mapping normalized roman key to original roman key.
- `deletes: HashMap<String, Vec<String>>`
  Mapping delete key to normalized original roman keys.
- `max_edit_distance: usize`
  Start with `2`.

Keep the existing call shape during the first implementation:

```rust
fn get(&self, query: &str, threshold: f64) -> Option<Vec<(f64, String)>>
```

Map edit distance to a score compatible with current callers:

```text
score = 1.0 - distance / max(query_len, candidate_len)
```

Return original roman forms through `exact` so downstream ranking behavior stays
as close as possible to the current index.

## Build Algorithm

For each roman key:

1. Normalize once.
2. Skip duplicates already present in `exact`.
3. Generate all delete keys up to edit distance `2`.
4. Insert the normalized original into each delete bucket.
5. Store normalized-to-original in `exact`.

Implementation notes:

- Deduplicate generated delete keys per source word.
- Deduplicate candidates during query.
- Keep deterministic ordering by sorting after scoring.
- Avoid external crates in the first pass unless a local implementation proves
  too costly or error-prone.

## Query Algorithm

For a query:

1. Normalize query.
2. Return exact match early when available.
3. Generate delete keys up to edit distance `2`.
4. Collect normalized candidates from matching delete buckets.
5. Compute existing Levenshtein `similarity(...)` against the normalized query.
6. Filter by the caller-provided `threshold`.
7. Sort by score descending, then candidate key ascending for stability.
8. Convert normalized candidates back to original roman forms with `exact`.

## A/B Strategy

Do not delete the current n-gram implementation immediately.

Recommended first shape:

```rust
enum SearchIndex {
    Ngram(NgramSearchIndex),
    SymSpell(SymSpellIndex),
}
```

or keep a private `NgramSearchIndex` plus a temporary constructor switch.

The first experiment can make SymSpell the default for full legacy data only
after direct tests pass. Phase-A should remain unchanged unless measured
separately.

## Tests

Add direct SymSpell tests covering:

- exact query returns the original roman key;
- one-edit typo returns expected candidates;
- two-edit typo returns expected candidates;
- duplicate roman entries do not duplicate returned candidates;
- returned order is deterministic;
- threshold filtering behaves like the old caller contract.

Run focused regression checks:

```bash
cargo test -p khmerime_core -- --test-threads=1
cargo test --test decoder_golden -- --test-threads=1
```

Golden diffs are possible because legacy fuzzy recovery can affect fallback
ordering. Do not update golden snapshots until the new behavior is reviewed.

## Timing Checks

Build the release bridge:

```bash
cargo build --release --bin khmerime_ibus_bridge
```

Probe startup:

```bash
(printf '{"cmd":"snapshot"}\n'; sleep 2; printf '{"cmd":"shutdown"}\n') \
  | target/release/khmerime_ibus_bridge --deferred-segmented-preview
```

Compare:

- `phase_a_session.end`
- `full_shared_data.build_legacy_data.search_index.end`
- `full_warmup.end`

Targets:

- phase-A first snapshot under `200 ms` in release;
- search-index stage meaningfully below current `200-300 ms`;
- total full warmup at or below `1 s`, if ranked-index cost allows.

## Rollout Decision

Accept SymSpell as default only if:

- release timing improves enough to matter;
- core tests pass;
- decoder golden diffs are absent or intentionally accepted;
- manual `make suggest QUERY=<roman>` checks for common typos still look good.

Keep SymSpell experimental if:

- timing improves but candidate ordering changes are broad;
- typo recovery regresses for common Khmer romanization inputs.

Reject or defer if:

- build cost is not meaningfully lower than the n-gram index;
- query quality requires duplicating weighted-span ranking logic;
- it pushes the implementation toward a second parallel decoder.

## Follow-Up Questions

- Should delete distance stay at `2`, or should long roman tokens use `3`?
- Should candidate ranking include lexicon target frequency before returning to
  `LegacyData::suggest(...)`?
- Should the build produce a compact serialized index later, instead of building
  delete buckets at runtime?
