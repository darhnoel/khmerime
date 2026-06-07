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

## iOS Keyboard Extension Phase 1

- status: planned
- scope: `adapters/ios-keyboard/`, `docs/platforms/ios.md`, `Makefile`
- spec: `docs/adr/0006-uniffi-swift-rust-bridge-for-ios.md`, `docs/platforms/ios.md`
- goal: Build a functional iOS custom keyboard extension with QWERTY layout, horizontal candidate strip, expandable segment panel, and UniFFI Rust bridge to `khmerime_session`.
- validation: `cargo check -p khmerime_ios_keyboard`; `make platform-build-ios`; Xcode simulator smoke checklist in `docs/platforms/ios.md`
- notes: UniFFI bridge (ADR-0006). `KhmerIMESession` is the Swift-visible session handle. `swift/` folder holds `KhmerIME.xcodeproj` with `KhmerIME` host app and `KhmerIMEKeyboard` extension targets. XCFramework built to `adapters/ios-keyboard/swift/Frameworks/` (gitignored). `Left`/`Right` are not keyboard buttons — the adapter fires them internally when the user taps a segment in the expanded panel. `Digit(n)` selects only; `Enter` is the explicit commit. `Tab`/`Escape` (Segment Edit Mode) deferred to a later milestone.

## ime_session.rs Module Split

- status: done
- scope: `crates/session/src/ime_session.rs` (~2,271 lines) → new sibling modules under `crates/session/src/`
- spec: `specs/structure/module-boundaries.md` (Native IME Boundaries), `CONTEXT.md`
- goal: Behavior-preserving extraction of the oversized `ime_session.rs` into focused modules named from the `CONTEXT.md` glossary. `ImeSession` and top-level `process_command()` / `process_key_event()` stay in `ime_session.rs`.
- validation: `cargo fmt --all`; `cargo test -p khmerime_session`; `cargo test -p khmerime_linux_ibus --test ibus_bridge_protocol`; broader `cargo test` if shared behavior moved substantially
- notes: Settled module names (grill-with-docs): `adapter_contract.rs` (NativeKeyEvent, SessionCommand, SessionSnapshot, SessionResult, InputMode, CursorLocation, HistoryStore, SegmentedPreviewMode, ImeSessionOptions); `session_snapshot.rs` (`snapshot()` + render projection); `segmented_session.rs` (glossary term); `segment_edit_mode.rs` (glossary term); `commit_rules.rs` (`commit_selected_or_raw`, `segmented_outputs`, `visible_candidate_outputs`, `hidden_commit_fallback`, `selected_or_raw_fallback`, `visible_refined_phrase_segments_for`, `refined_phrase_segments_for` — the three Commit Rules + raw floor, now a `CONTEXT.md` glossary entry). Rejected software-generic names: `commit_policy`, `commit_text`, `composition_commit`, `commit_resolution`, `commit_precedence`, `commit_source`, `contract`, `segmented`. No semantic / snapshot-JSON / Segment-Edit / history-learning changes during extraction.

## Build-Time Dictionary Image For System Lexicon

- status: in_progress
- scope: `crates/core/build.rs`, `crates/core/src/roman_lookup/`, `crates/core/src/decoder/weighted_span.rs`, `adapters/linux-ibus/src/bin/khmerime_ibus_bridge.rs`, tests/golden coverage
- spec: `docs/adr/0004-build-time-dictionary-image-for-system-lexicon.md`, `specs/structure/module-boundaries.md`, `specs/structure/verification-surfaces.md`
- goal: Replace heap-heavy system **Lexicon** structures in **SharedTransliteratorData** with a build-time **Dictionary Image** while preserving current IME behavior. First serious milestone is Linux **Bridge** steady-state RSS under 60 MB after full warmup and one `snapshot` command.
- validation: `cargo fmt --all`; `cargo test -p khmerime_core`; `cargo test -p khmerime_session`; `cargo test --test decoder_golden`; `cargo test -p khmerime_linux_ibus --test ibus_bridge_protocol`; bridge RSS harness after `full_warmup.end` and one `snapshot` command; representative `target/release/lookup_cli segmented <roman>` and `target/release/lookup_cli suggest <roman>` checks
- notes: Keep `LegacyData` as the internal behavior facade for `Transliterator`, decoders, session, CLI, Dioxus, and the **Bridge**. Remove heap-owned lookup maps only for the default system **Lexicon** path. Custom CSV/TSV and other runtime lexicon paths stay heap-backed until a later slice. Use an embedded `include_bytes!` image with zero-copy internal views first, but keep the manual little-endian format mmap-compatible. Leave **Learned History** TSV/HashMap unchanged. Retire the bincode `SharedTransliteratorData` cache for the image-backed default path; do not mmap the current bincode cache.

### Current Measurements And Target

- Installed **Bridge** cache-hit steady state: about 168 MB RSS, almost entirely private anonymous heap.
- Current release full-build steady state: about 188 MB RSS; the extra ~20 MB is likely allocator-retained build-path temporary memory.
- Isolated remaining contributors: heap-owned `LegacyData` lookup maps are about 82 MB; `SearchIndex(Ngram)` is about 23 MB; SymSpell is larger and not a memory-reduction path.
- First image-backed legacy lookup slice reduced the release **Bridge** to about 107 MB RSS after `full_warmup.end` and one `snapshot` command. The next target is the heap-backed legacy `Search Index` and remaining ranked/corpus/entry duplication while preserving exact output parity across public `Transliterator` and session behavior.

### Storage Boundary

- `LegacyData` remains the facade; storage becomes an internal implementation detail.
- Use a storage enum or equivalent internal boundary so default system data can be image-backed while custom/runtime data remains heap-backed.
- Public behavior methods must continue to define the compatibility contract: `suggest()`, `exact_targets()`, `exact_match_roman_variants()`, `best_prefix_consumption()`, `next_word_suggestions()`, and `infer_next_word_context_suffix()`.
- Image-backed helpers should expose behavior-level queries rather than raw sections: roman to targets, normalized roman to exact targets, target to roman variants, target frequency, roman to normalized roman, prefix to roman candidates, and all roman keys when fallback requires them.

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

5. Legacy exact/prefix/reverse slice:
   - Extend the **Dictionary Image** with the heap-owned `LegacyData` lookup surfaces: `by_roman`, `by_normalized`, `by_target`, `target_frequency`, `roman_normalized`, and `roman_prefix_index`.
   - Add dual-read parity tests against heap-backed maps before routing production behavior through the image.
   - Route `LegacyData::suggest()`, `exact_targets()`, `exact_match_roman_variants()`, and `best_prefix_consumption()` through storage helpers.
   - Remove the equivalent heap maps only for the default system **Lexicon** after public output parity holds.
   - Temporarily keep current `SearchIndex(Ngram)` until flat exact/prefix/reverse suggestions are behavior-equivalent.

6. Legacy `Search Index` slice:
   - Move `SearchIndex(Ngram)` exact/items/gram tables into sorted image sections or replace it with equivalent image-backed range queries.
   - Preserve current threshold and ordering behavior before removing the heap-backed Ngram index.
   - Do not switch to SymSpell for memory reduction; measured SymSpell RSS is substantially larger.

7. Default cache retirement slice:
   - Bypass the bincode `SharedTransliteratorData` cache for the default image-backed path.
   - Keep cache code available only for heap-backed custom/runtime paths if those paths still need it.
   - Remove the cache-hit `attach_dictionary_image()` workaround once the default path no longer deserializes `LegacyData`.

8. Phase A/Phase B cleanup:
   - Redefine Phase A as a cheap view over the same **Dictionary Image**, not a separate heap-built **SharedTransliteratorData**.
   - Measure whether Phase B can become lightweight lazy decoder/helper initialization.
   - Re-check allocator residue after full engine install.

### Parity Gate

- Exact output parity is required across public `Transliterator` and session surfaces, not byte-for-byte internal structure parity.
- Focused parity tests should compare image-backed default data with heap-backed data for representative `suggest()`, `exact_match_targets()`, `exact_match_roman_variants()`, `best_prefix_consumption()`, and segmented-session flows.
- Required test gate for slices that route behavior: `cargo fmt --all`; `cargo test -p khmerime_core`; `cargo test -p khmerime_session`; `cargo test --test decoder_golden`; `cargo test -p khmerime_linux_ibus --test ibus_bridge_protocol`.

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
