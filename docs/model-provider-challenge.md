# Bring your own model — the span-proposal seam

KhmerIME's decoder is deterministic: a lexicon, a weighted-span search, and a scoring
model you can read end to end. That is a deliberate choice — it is debuggable, it runs in
milliseconds on a phone, and it never invents a word that isn't in the dictionary.

But romanized Khmer is genuinely ambiguous. `kmean` can be កម្មាន, គ្មាន, or ក្មាន
depending on what the sentence is about, and no amount of dictionary lookup settles that.
Context does. Which makes this a good problem for a learned model — and the engine has a
seam where one can be plugged in.

**This document is an invitation to plug something in.**

## The seam

The decoder can ask an external provider for *span proposals*: given the raw roman input,
which character ranges are probably words, and what Khmer might they be? Everything about
that provider is behind a trait, so the engine neither knows nor cares what produces the
answer — a neural model, a statistical n-gram table, a rules engine, a lookup against a
corpus you scraped.

The contract:

- **The engine stays in charge.** Proposals are *suggestions*. The decoder still scores
  them against the lexicon, still applies the commit rules, and still refuses to commit a
  candidate that isn't real Khmer. A bad provider degrades ranking; it cannot corrupt
  output.
- **Nothing blocks a keystroke.** The provider runs on a debounced pass, off the typing
  hot path, under a latency budget. Miss the budget and the deterministic result stands.
  See [ADR-0005](adr/0005-commit-refiner-keeps-a-latency-budget.md).
- **Absent by default.** With no provider registered, the flag flips and nothing happens —
  the engine behaves exactly as it does today. See
  [ADR-0016](adr/0016-runtime-model-provider-behind-a-lexicon-verified-marker.md).
- **Candidates carry provenance.** Anything a provider influenced is marked, so the UI can
  tell the user which suggestions came from a model and which are lexicon-verified.

## Where to look

| What | Where |
|---|---|
| The provider trait and registration | `crates/core/src/decoder/` (`SpanProposalMode`, the provider seam) |
| How an adapter turns the mode on | `KhmerImeSession::set_model_mode` |
| The debounced, off-hot-path refine | `ModelRefiner` (Android), the visible refiner (iOS/macOS) |
| The arming stub each platform swaps | `AiModelArming.swift` (iOS) — a no-op in this repo |
| Why it is a runtime seam, not a fork | [ADR-0016](adr/0016-runtime-model-provider-behind-a-lexicon-verified-marker.md) |

`AiModelArming.armIfNeeded()` returning `false` in this repository is not an oversight.
It is the socket. A build that has a provider replaces that one function; everything else
is the code you are reading.

## The challenge

Khmer is under-served by input methods, and the hard part is not the keyboard — it is
knowing which of several valid spellings a person meant. If that interests you:

1. **Train something.** Character-level seq2seq, a transliteration transducer, a small
   context model over committed phrases — the seam does not care.
2. **Implement the provider trait** and register it at startup.
3. **Measure it.** The repo ships golden snapshots and a shadow-eval harness
   (`make shadow-eval QUERIES=…`) so you can prove a change helps before shipping it.
   Exact-match@1 on held-out phrases is the number to beat.
4. **Mind the budget.** A keyboard that stutters is worse than one that guesses wrong.
   Fitting a model into a mobile keyboard extension's memory ceiling is most of the work.

An AI-embedded build of this keyboard exists and ships on the app stores; its model is not
open source. The **seam is**, deliberately — so anyone can build a better one, or a
different one, or one for another language entirely, without asking permission or forking
the engine.

If you build something, open an issue. We would like to see it.
