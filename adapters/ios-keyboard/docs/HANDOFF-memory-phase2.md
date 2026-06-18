# Handoff: iOS keyboard memory reduction — Phase 2 (and 3)

## Prompt for codex (paste this to start)

> Continue the iOS keyboard memory-reduction work on the `ios` branch. The custom
> keyboard intermittently falls back to Apple's "km Khmer" keyboard because the
> extension is jettisoned for memory (confirmed: it idles at ~46 MB with only ~30 MB
> headroom under the iOS 77 MB extension cap, and critical system pressure kills it).
> Root cause + plan: `adapters/ios-keyboard/docs/session-init-memory.md`. Phase 1
> (ranked entry table → zero-copy dictionary image, −14 MB heap) is DONE and
> device-verified. Execute **Phase 2**: move the decoder's 7 n-gram maps into the
> dictionary image so they can be dropped from the heap (~−14 MB). Follow the detailed
> spec in `session-init-memory.md` (the "N-gram postings + corpus stats → image
> sections" bullet) and the rules below. Work incrementally, keep the golden snapshot
> green at every step, and re-measure with the harness. Then do Phase 3 (composer
> table → image, ~−13 MB) the same way. When footprint is comfortably low, remove the
> two TEMPORARY probes (see Cleanup).

## Where things stand

- Branch `ios`. Relevant commits: `38b95db` (diagnosis), `b1c241c` (image alias_keys),
  `94ce8f9` (Phase 1: ranked entries from image), `f6e293d` (Phase 2 spec).
- Device measurement (iPhone, Release): launch 4.3 MB → **after session ~46 MB,
  headroom ~30 MB**. Was 62.6/14.4 before Phase 1. Still jetsam'd under critical pressure.
- Target: footprint < ~25 MB, headroom > ~50 MB. Phase 2 (~−14 MB) → ~33 MB; Phase 3
  (~−13 MB) → ~20 MB.

## The data flow you're changing

- The dictionary image is a zero-copy `include_bytes!` binary built at compile time.
  - Format constants + schema: `crates/core/src/roman_lookup/dictionary_image_format.rs`
    (currently schema v4, `SECTION_COUNT = 20`).
  - Builder: `crates/core/build.rs` → `compile_dictionary_image(entries, corpus_stats)`.
  - Reader: `crates/core/src/roman_lookup/dictionary_image.rs` (`DictionaryImageView`).
  - The `.bin` lives in `OUT_DIR` (gitignored) and is rebuilt by `build.rs`.
- The heap decoder data is `LegacyData` (`crates/core/src/roman_lookup/legacy_data.rs`),
  which holds `RankedLexicon` (`ranked_lexicon.rs`, struct in `types.rs`). When the image
  is present (default system lexicon), heap structures are progressively dropped and the
  decoder reads from the image instead.
- The decoder is `crates/core/src/decoder/weighted_span.rs`.

## Phase 2 = the 7 n-gram maps (exact spec in session-init-memory.md)

Maps in `RankedLexicon`, used by `weighted_span::context_delta` / `pos_delta`:
- Unigrams (`HashMap<String,u32>`): `word_unigrams`, `corpus_word_unigrams`,
  `corpus_surface_unigrams`, `tag_unigrams`.
- Bigrams (`HashMap<(String,String),u32>`): `word_bigrams`, `corpus_word_bigrams`,
  `tag_bigrams`.

Plan: add 7 image sections (unigram = sorted `(string_id,u32)`, reuse
`STRING_U32_RECORD_LEN`; bigram = sorted `(id1,id2,u32)`, new `BIGRAM_RECORD_LEN=12`),
bump schema v4→v5 and `SECTION_COUNT` 20→27, write generic compile + reader helpers,
add image-or-heap count accessors on `LegacyData`, rewire `context_delta`/`pos_delta` to
take `&LegacyData`, and clear the 7 maps on `ranked` when the image is present.

## The #1 gotcha (learned the hard way in Phase 1)

**The image must reproduce the PRODUCTION (real-corpus) ranked values, not the
default-corpus ones.** Ranked applies corpus-derived computation:
- entry frequency = `entry.frequency.max(corpus_unigram).max(1)` (already replicated in
  the builder — see `compile_dictionary_image`).
- boundary tags from `boundary_tags_for_target` (already replicated).
- the n-gram maps themselves: `word_unigrams`/`word_bigrams` are built from entry targets
  over ALL entries (do NOT skip empty-normalized ones); `corpus_*`/`tag_*` come straight
  from the khpos `CorpusStats`.

So in the validation test, build `ranked` with the **real** khpos stats
(`parse_compiled_khpos_stats(DEFAULT_COMPILED_KHPOS_STATS)`), NOT `CorpusStats::default()`,
and assert every image count equals the ranked map count. Model: the existing test
`dictionary_image_matches_ranked_retrieval_indexes` in `roman_lookup/mod.rs`.

Build plumbing: `compile_khpos_stats` already computes `word_bigrams`, `tag_unigrams`,
`tag_bigrams` — thread them into `BuildCorpusFrequencyStats` (it already carries
`word_unigrams`, `surface_unigrams`, `dominant_word_tags`; extend it like Phase 1 did for
tags). Build `word_unigrams`/`word_bigrams` in `compile_dictionary_image` from entry
targets exactly as `ranked_lexicon.rs` does.

## Working rules

- **Golden is the gate.** After any decoder/image change:
  `cargo test --test decoder_golden` (workspace-level, ~60s). It must stay green —
  this is a memory change with ZERO behavior change. `Golden-Changed: no`.
- **Incremental + safe.** Land the image sections + validation test FIRST (no decoder
  change, build stays green, golden unaffected), commit, THEN swap the decoder + drop the
  maps, commit. A half-applied multi-section format change breaks the whole image.
- **Measure each step** with the harness:
  `cargo test -p khmerime_core --features no-search-index --test memory_breakdown -- --nocapture --test-threads=1`
  (look at the `iOS path TOTAL` line).
- Also run `cargo test -p khmerime_core --lib` and `cargo test -p khmerime_session`.
- Commit style: conventional commits, NO `Co-Authored-By` trailer (repo rule),
  and do NOT commit `data/**` or `note.md`.

## Phase 3 (after Phase 2): composer table → image (~−13 MB)

The composer (`crates/core/src/composer/`, `ComposerTable::from_entries`) is built from
`legacy.entries()` and is ~13 MB. Same approach: serialize it into image section(s),
read via views, build it from the image when present. Larger/structurally different from
the n-gram maps — scope it carefully and keep golden green.

## Cleanup (when footprint is low enough / fallbacks stop)

Two TEMPORARY diagnostics to remove:
1. `KeyboardViewController.logMemory(_:)` + its 3 call sites + `import os` (the `MEM`
   probe) in `adapters/ios-keyboard/swift/KhmerIMEKeyboard/Controller/KeyboardViewController.swift`.
2. `crates/core/tests/memory_breakdown.rs` (the counting-allocator harness) — or keep a
   trimmed RSS-guard test if you want a regression guard.

## Verifying on device (the real test)

Rebuild the iOS extension (Release) → Console.app → filter `PROCESS contains Khmer` →
switch to the KhmerIME keyboard → read the three `MEM` lines. Success = `after session`
footprint well under the previous 46 MB and headroom comfortably above 30 MB, and no more
fallback to Apple's km Khmer keyboard under heavy multitasking/backup pressure.
