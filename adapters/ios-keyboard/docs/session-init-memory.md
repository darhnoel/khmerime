# iOS keyboard fallback to the system Khmer keyboard — root cause

## Symptom

Intermittently, switching to the KhmerIME keyboard shows **Apple's built-in "km Khmer"
keyboard** instead of our custom layout. It correlates with the device having been
under load (other apps, multitasking, backups).

## Root cause (confirmed on device)

The extension is **jettisoned for memory** — not a code crash, not a watchdog.

Evidence chain:

1. The fallback is Apple's system keyboard → iOS killed our extension and fell back to
   the other installed Khmer keyboard. (`Keyboards: com.khmerime.KhmerIME.Keyboard, km Khmer`
   in the JetsamEvent confirms both are installed.)
2. Device logs show `bug_type 211` (JetsamEvent) and `ATXMemoryPressureMonitor … type:
   critical` — a memory-pressure event, not a crash report.
3. On-device instrumentation (`os_proc_available_memory()` + `phys_footprint` logged in
   `KeyboardViewController.viewDidLoad`) measured:

   | phase | footprint | headroom |
   |---|---|---|
   | launch | 4.5 MB | 72.5 MB |
   | after `KeyboardSession()` | **62.6 MB** | **14.4 MB** |
   | after layout | 63.4 MB | 13.6 MB |

   The extension cap is ~77 MB (4.5 + 72.5). After building the session we sit at
   **62.6 MB with ~14 MB of headroom**, and iOS fires `Received memory warning`
   immediately. We don't blow our own per-process limit (hence no `per-process-limit`
   crash report), but with ~14 MB to spare, **system-wide pressure tips us over → jetsam
   → fallback.**

This is *not* the original `SearchIndex` bug (that is disabled via `no-search-index`).
It's the next-largest offender: session init builds the decoder's statistical models on
the heap.

## Where the ~58 MB goes (measured)

A counting-allocator harness (`crates/core/tests/memory_breakdown.rs`, run with
`--features no-search-index`) broke down the ~53 MB heap of `from_default_data_with_config`:

| structure | heap | droppable? |
|---|---|---|
| lexicon `entries` (`Vec<Entry>`) | 4.2 MB | yes, but tiny |
| lookup maps | ~1.2 MB | already minimized by the dictionary image |
| dictionary image | ~0 MB | zero-copy ✓ |
| khpos corpus n-gram stats | ~14.5 MB | decoder scoring |
| ranked lexicon (freq + indexes) | ~16 MB | decoder scoring; entries mirror the image |
| composer / decoder table | ~13 MB | decoder |
| next-word stats | ~4.6 MB | decoder |

**~47 of ~53 MB is the decoder's statistical "brain"** (corpus n-grams, ranked lexicon,
composer, next-word). The lexicon `entries` everyone reaches for first is only 4.2 MB.
The lookup maps are already served from the embedded zero-copy `DictionaryImageView`.

## Why deferring/async init does NOT fix it

The 62.6 MB is **resident steady-state** for the whole time the keyboard is up, not a
launch-only spike. Deferring session construction only delays reaching the cliff; under
pressure we still get jettisoned. The only fix is **lowering resident memory.**

## Fix direction (chosen) — executes the unfinished slice of ADR 0004

**Extend the zero-copy dictionary-image approach to the decoder's statistical models** so
the decoder reads corpus n-grams / ranked lexicon / composer mmap'd from the embedded
binary instead of heap-parsing them. No suggestion-quality loss; the data is the same,
only its residency changes.

This is **not a new architectural decision** — [ADR 0004](../../../docs/adr/0004-build-time-dictionary-image-for-system-lexicon.md)
already adopted the build-time Dictionary Image and explicitly named the next routing
targets as "the ranked entry table, exact/alias indexes, ngram postings, and corpus
stats." The lookup-map slice has landed (those are already served from the image); the
**decoder-model slice 0004 named is the unfinished part**, and the iOS 77 MB cap now
prioritizes it.

### Phased plan (each phase measurable via the harness, golden-test guarded)

1. **Ranked entry table → image. ✅ DONE — measured −14.2 MB heap (52.8 → 38.6 MB).**
   - alias_keys section added to the image (schema v4) + `entry_alias_keys` accessor
     (commit b1c241c).
   - The image entry table must match the *production* (real-corpus) ranked table, not
     the default-corpus one: the heap computes **corpus-adjusted frequency**
     (`entry.frequency.max(corpus_unigram).max(1)`) and **boundary tags**
     (`boundary_tags_for_target`). Both are now replicated in the builder —
     `BuildCorpusFrequencyStats` carries `dominant_word_tags`, and
     `compile_dictionary_image` computes effective frequency + first/last tags per
     entry. The equivalence test now builds `ranked` with the real khpos stats and
     asserts frequency/tags/alias_keys all match.
   - `RankedEntryView` (heap | image) + `LegacyData::ranked_entry`; the `weighted_span`
     decoder (`score_span_candidate`, `compare_retrieval_hits`) reads entries through it.
   - `ranked.entries` is dropped when the image is present. Golden snapshot unchanged.
2. **N-gram postings + corpus stats → image sections.** The ~14.5 MB khpos corpus stats
   and the word/tag n-gram maps become offset/range sections read via views.
3. **Composer table → image.** The ~13 MB composer/decoder table.
4. **Next-word stats → image.** The ~4.6 MB next-word tables.

`LegacyData` stays the internal facade for all callers (decoder, session, CLI, Dioxus,
Bridge); only the residency of the default-system data changes underneath. Per phase:
add the image section(s) + schema-version bump in the core-owned builder, read via views,
run the golden decoder tests (behavior must not change), and re-measure footprint with the
harness + the on-device `MEM` probe.

Target: session-init footprint < ~35 MB, headroom > ~40 MB.

Target: drop session-init footprint from ~62 MB to a level that leaves comfortable
headroom (e.g. footprint < ~35 MB, headroom > 40 MB) so normal system pressure no longer
jettisons us.

## Measurement harness (temporary)

- `KeyboardViewController.logMemory(_:)` — on-device footprint/headroom at launch. Marked
  TEMPORARY; remove once the fix is verified on device.
- `crates/core/tests/memory_breakdown.rs` — host-side per-stage heap breakdown. Remove
  after the refactor; or keep a trimmed RSS-guard test (see ADR).
