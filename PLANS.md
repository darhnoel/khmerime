# Plans

This file is the lightweight planning ledger for work that spans more than one
small edit or changes a maintained boundary.

Use it to record:

- active or next structural work
- the reason for the work
- the files or subsystems expected to move
- the required verification before the work can be considered done

Do not use it for:

- durable architecture rules
- module ownership
- behavior contracts that should live under `specs/`

Those belong in `AGENTS.md` or `specs/`.

## Workflow

1. Add or update a plan entry before a non-trivial change.
2. Link the plan to the relevant spec file when one exists.
3. Remove or archive stale plan details once the durable rule has been folded into
   code, tests, or specs.

## Status Labels

- `planned`
- `in_progress`
- `blocked`
- `done`

## Template

```md
## <short title>

- status: planned|in_progress|blocked|done
- scope: <subsystems or files>
- spec: <relevant spec path or none>
- goal: <one or two sentences>
- validation: <commands or tests to run>
- notes: <optional constraints, unknowns, or rollout details>
```

## Current Entries

## Build-Time Dictionary Image For System Lexicon

- status: planned
- scope: `crates/core/build.rs`, `crates/core/src/roman_lookup/`, `crates/core/src/decoder/weighted_span.rs`, `adapters/linux-ibus/src/bin/khmerime_ibus_bridge.rs`, tests/golden coverage
- spec: `docs/adr/0004-build-time-dictionary-image-for-system-lexicon.md`, `specs/structure/module-boundaries.md`, `specs/structure/verification-surfaces.md`
- goal: Replace heap-heavy system **Lexicon** structures in **SharedTransliteratorData** with a build-time **Dictionary Image** while preserving current IME behavior. First serious milestone is Linux **Bridge** steady-state RSS under 60 MB after full warmup and one `snapshot` command.
- validation: `cargo fmt --all`; `cargo test -p khmerime_core`; `cargo test -p khmerime_session`; `cargo test --test decoder_golden`; bridge RSS harness with and without `KHMERIME_DISABLE_SHARED_DATA_CACHE=1`; representative `target/release/lookup_cli segmented <roman>` checks
- notes: Use an embedded `include_bytes!` image with zero-copy internal views first, but keep the manual little-endian format mmap-compatible. Leave **Learned History** TSV/HashMap unchanged. Do not mmap the current bincode cache.

### Dictionary Image Stages

1. Coexistence slice:
   - Add core-owned image builder, validator, and internal `DictionaryImageView`.
   - Generate a build-time image from the default system **Lexicon** and required ranked fields.
   - Use numeric string IDs from day one, with `u32::MAX` as the missing-string sentinel.
   - Validate exact and alias lookup against current `RankedLexicon` without routing production behavior through the image.

2. Weighted-span retrieval slice:
   - Route weighted-span exact and alias retrieval through `DictionaryImageView`.
   - Preserve current candidate set, ordering, and decoder outputs exactly.
   - Keep corpus stats and legacy flat `SearchIndex` on current structures for this slice.

3. Weighted-span postings slice:
   - Move weighted-span 2-gram postings from `RankedLexicon.gram_index` into packed image posting ranges.
   - Remove the equivalent heap map only after golden and representative CLI outputs match.

4. Corpus stats slice:
   - Move word/surface/tag unigram and bigram stats into ID-keyed image tables.
   - Keep `corpus_word_bigrams` behavior measurable because it is hit in normal segmented typing but not dominant.

5. Legacy flat suggestion slice:
   - Move `LegacyData::suggest()` exact/prefix/search surfaces onto image-backed indexes.
   - Temporarily keep current `SearchIndex` until flat suggestions are behavior-equivalent.
   - Remove or shrink duplicated legacy maps after the image-backed path passes tests.

6. Phase A/Phase B cleanup:
   - Redefine Phase A as a cheap view over the same **Dictionary Image**, not a separate heap-built **SharedTransliteratorData**.
   - Measure whether Phase B can become lightweight lazy decoder/helper initialization.
   - Re-check allocator residue after full engine install.

## Ubuntu Native IBus Switching (Mozc-like v1)

- status: in_progress
- scope: `src/ime_session.rs`, `src/bin/khmerime_ibus_bridge.rs`, `src/history_store.rs`, `scripts/*ibus*`, docs/spec updates
- spec: `specs/structure/module-boundaries.md`, `specs/structure/verification-surfaces.md`
- goal: Add Ubuntu-native IBus source switching integration around the existing KhmerIME transliterator flow with developer-local install scripts and desktop history persistence.
- validation: `cargo fmt --all`; `cargo test`; `bash scripts/smoke_test_ibus_engine.sh`
- notes: v1 keeps IBus-only scope and system input-source switching (no internal direct/khmer toggle).

## Oversized Engine/Runtime Split (Roman + Main)

- status: done
- scope: `src/roman_lookup/`, `src/main.rs`, `src/engine_registry.rs`, `src/startup_fetch.rs`, `src/startup_signals.rs`
- spec: `specs/structure/module-boundaries.md`, `specs/structure/verification-surfaces.md`
- goal: Split oversized engine/runtime files into focused internal modules while preserving behavior and existing public API.
- validation: `cargo fmt --all`; `cargo test`; `cargo test --test decoder_golden`; `.venv/bin/pytest tests/test_web_ui.py`; `cargo run --features cli --bin lookup_cli -- suggest jea`; `cargo run --features cli --bin lookup_cli -- suggest tver`
- notes: Keep module-local `roman_lookup` tests in place for this pass. Existing baseline failures remain unchanged (`cargo test` has 2 known failures; `decoder_golden` has 1 known mismatch).

## Manual Character Typing Mode (Phase 1)

- status: done
- scope: `src/decoder/manual_character_typing.rs`, `src/ui/editor.rs`, `src/ui/components/`, `src/ui/storage.rs`, `src/main.rs`
- spec: `specs/structure/module-boundaries.md`, `specs/structure/verification-surfaces.md`
- goal: Add an explicit manual character typing mode that supports guided base-consonant/vowel composition without auto-switching from normal mode, and optionally save confirmed manual mappings for reuse.
- validation: `cargo fmt --all`; `cargo test`
- notes: Keep existing word suggestion pipeline intact and integrate manual mode through explicit UI mode selection.

## Khmer Chosen-Word Sequence Prediction (Word Mode v1)

- status: planned
- scope: `src/ui/storage.rs`, `src/ui/editor/`, `src/main.rs`
- spec: `specs/structure/module-boundaries.md`, `specs/structure/verification-surfaces.md`
- goal: Persist Khmer chosen-word sequence history (bigram+trigram) and use session context to rerank Word mode suggestions without changing manual mode behavior.
- validation: `cargo fmt --all`; `cargo test`; `.venv/bin/pytest tests/test_web_ui.py`
- notes: Detailed execution plan is documented in `plans/khmer-word-sequence-history.md`.
