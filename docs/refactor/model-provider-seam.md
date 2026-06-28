# Refactor plan — pluggable model-provider seam

**Goal:** land the span-proposal *seam* on `dev` (public) so any AI/ML model can plug in later,
while the actual model (byT5 + candle) stays on `spike` and **never reaches dev**. dev gets an
empty, inert socket; spike fills it.

This doc lives on `spike` and is **not** one of the files brought to dev (see Phase 2).

## Locked decisions

- **Name:** the seam mode/feature is `Model` (matches the public "we use an AI model" announcement);
  the trait stays generic `SpanProposalProvider` so non-model providers can plug in later.
  | thing | name |
  |---|---|
  | trait | `SpanProposalProvider` *(unchanged)* |
  | mode | `SpanProposalMode::Model` |
  | feature | `model-provider` |
  | env | `KHMERIME_SPAN_PROPOSALS=model` |
  | gated module | `mod model_provider` *(spike-only file; wraps byT5)* |
  | dev default | `Model => None` → pure lexicon |
- **Concealment Level 1:** cfg-gated module in the same crate; `candle` is an *optional* dep behind
  the feature. dev's `Cargo.toml` shows `candle (optional)` — acceptable given the announcement.
  (Level 2 = separate crate + runtime injection; not chosen.)
- **Inert-by-default:** with no provider, zero new work on any path (proven below).

## Why dev still builds without the model files

A `#[cfg(feature = "model-provider")] mod model_provider;` is dropped *before* file resolution when
the feature is off — so `model_provider.rs` / `byt5_t5.rs` need not exist on dev. Verified:

```
DEV   (feature off, files absent):  build ok, provider = None
SPIKE (feature on):                 error: file not found   ← spike HAS the files
```

## File map (end state)

```
DEV (public)                                   SPIKE = dev + these (never cherry-picked):
crates/core/src/decoder/                       crates/core/src/decoder/
  span_proposal.rs  trait + StaticTest +         model_provider.rs   byT5 impl  ← only "byt5" here
                    provider_for_mode(Model⇒None) byt5_t5.rs          candle T5
  config.rs         SpanProposalMode::Model      crates/core/Cargo.toml: candle (optional, default-off)
  weighted_span.rs  decode gated on is_some()    adapters/macos-imk/Cargo.toml: feature = model-provider
  mod.rs  #[cfg(feature="model-provider")]
          mod model_provider;   ← file ABSENT on dev, still compiles
crates/session/...  lock-free refine (generic)
adapters/macos-imk/ debounced refiner, armed ONLY when a provider is present
```

## Invariants (CI-enforced, not hopes)

1. **`grep -ri byt5` over the dev-bound files = empty.** byT5 appears only in `model_provider.rs` +
   `byt5_t5.rs` (spike-only).
2. **Inert when unplugged:** dev default builds with `span_proposer == None` *and* the debounced
   refiner disarmed. A criterion bench asserts decode latency (seam + provider `None`) is within
   noise (≤1%) of baseline.
3. **No behavior drift:** the decoder golden test is unchanged with provider `None`.

## Phase 1 — generalize *in place* on spike

Each commit is green: `cargo build` (no feature) **and** `cargo build --features model-provider`
**and** `cargo test` all pass. No behavior change; byT5 still works on spike.

| # | commit | files | verify |
|---|---|---|---|
| C1 | `refactor: rename span-proposal mode Byt5 → Model` | `config.rs`, `span_proposal.rs` (match arm), `adapters/macos-imk/src/lib.rs` (env value), tests | macOS byT5 path works with `KHMERIME_SPAN_PROPOSALS=model` |
| C2 | `refactor: generic feature + module names` | feature `byt5`→`model-provider` (core + macos `Cargo.toml`); `byt5_provider.rs`→`model_provider.rs`; cfg-gate `mod model_provider` + `mod byt5_t5`; cfg-split `provider_for_mode` (`Some(load)`/`None`) | both builds compile; no-feature build → provider `None` |
| C3 | `build: make candle an optional dep behind model-provider` | `crates/core/Cargo.toml` | default build does **not** compile candle; `--features model-provider` does |
| C4 | `refactor: scrub byt5 from the generic seam files` | comments/strings in `span_proposal.rs`, `weighted_span.rs`, `segmented_session.rs`, `adapters/macos-imk/src/lib.rs` | `grep -ri byt5` over those files = empty |
| C5 | `perf: arm the debounced refiner only when a provider is present` | `adapters/macos-imk/src/lib.rs` (refine trigger), maybe `weighted_span.rs` | with no provider, the pause runs no refine (log/test) |
| C6 | `test: seam inert-when-unplugged guard + latency bench` | `crates/core/tests/…`, `benches/…` | guard test + bench green; bench delta ≤1% |

After Phase 1: the generic seam files contain **zero "byt5"**; byT5 is quarantined to
`model_provider.rs` + `byt5_t5.rs`.

## Phase 2 — land the socket on dev (the split)

Do **not** cherry-pick mixed commits (they touch the model files). Instead bring the *final state*
of the generic files only:

```bash
git switch -c feature/model-provider-seam origin/dev
# bring ONLY the generic seam files from spike (NOT model_provider.rs / byt5_t5.rs):
git checkout spike/model-span-search -- \
  crates/core/src/decoder/span_proposal.rs \
  crates/core/src/decoder/config.rs \
  crates/core/src/decoder/weighted_span.rs \
  crates/core/src/decoder/mod.rs \
  crates/core/Cargo.toml \
  crates/session/src/{segmented_session,ime_session,commit_rules,lib}.rs \
  adapters/macos-imk/src/lib.rs \
  adapters/macos-imk/Cargo.toml
cargo build && cargo test               # dev: cfg'd mod → absent file → compiles; StaticTest proves seam
grep -ri byt5 crates/ adapters/ || echo "clean: no byt5 on dev"
cargo test --test decoder_golden        # unchanged
git commit -m "feat(decoder): pluggable model-provider seam (inert without a model)"
# open PR to dev
```

dev now has: the trait, `StaticTest`, `Model => None`, the lock-free refine, and the disarmed
refiner. Plugging a model later = add `model_provider.rs` (impl the trait) + flip the feature.

## Rollback

- Phase 1 commits are independent and revertable; byT5 keeps working at every step.
- Phase 2 is a single dev PR; revert = drop the PR. dev returns to exactly today (the seam is
  additive and inert).

## Open / follow-ups

- The Swift `refine_visible` dispatch (off main thread) is orthogonal — track separately.
- The 3 pre-existing `khmerime_session` segmentation test failures (lexicon-data refresh) exist on
  dev already; not caused by this refactor.
