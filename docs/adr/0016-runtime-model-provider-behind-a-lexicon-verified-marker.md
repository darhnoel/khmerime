# A runtime model provider may propose out-of-Lexicon candidates, marked as unverified

**Supersedes** `tools/variant-miner/docs/ADR-offline-only.md` (and its DECISIONS.md D5
"never at runtime" stance) for the runtime path.

The offline-only ADR made two arguments against a runtime neural fallback: a **memory**
argument (a 300M teacher + inference runtime cannot fit the engine's envelope — the iOS
extension's ~77 MB cap, the Linux **Bridge**'s <60 MB target) and a **hallucination-gate**
argument (the shipped engine stays pure lookup + fuzzy over human-reviewed **Lexicon** data, so a
bad model proposal can never reach a user). This ADR reverses both — deliberately, and with
different guardrails.

## What changes

1. **A runtime span-proposal seam.** `crates/core` exposes a generic `SpanProposalProvider` trait
   and `register_span_proposal_provider`. `SpanProposalMode::Model` resolves to the registered
   provider, or `None` when none is registered. **The public engine ships no provider** — the seam
   is inert by default, so the free/OSS build is byte-for-byte the pure lookup + fuzzy engine. A
   closed provider is registered only by a separate build.
2. **The Lexicon gate becomes a marker, not a filter.** Previously the **Weighted Span Decoder**
   rejected any model proposal whose output was not already a **Lexicon** target. That hard
   rejection is replaced by a per-candidate `lexicon_verified` flag (true iff every span is a real
   Lexicon target). Unverified model output now *reaches the user* — but is visibly marked (a red
   `✦` on the **Phrase Wheel** card / **Strip**), so it can never masquerade as human-reviewed data.
3. **Off the hot path.** The provider never runs on a keystroke. It runs only via the **Visible
   Refiner** on a debounced pause, and (on iOS) lazy-loads on first use and frees on focus-out.

## Why this is acceptable now

- **The memory argument is addressed at the artifact level, not by this ADR.** The runtime model is
  a distilled ~5 MB-class student, not the teacher — small enough that "does it fit" is an empirical
  per-platform question, not a categorical no. Whether it *actually* fits the 77 MB iOS extension is
  still being proven on-device; if it cannot, the resolution is to run it in the container app or to
  stay offline — not to weaken this ADR.
- **The hallucination gate is preserved, differently.** Trust is no longer "the engine only ever
  emits reviewed words"; it is "**anything not in the reviewed Lexicon is visibly unverified**." The
  user still sees only Lexicon words as trusted; the model's guesses are opt-in-looking suggestions,
  not silent authority. A trusted **Lexicon** candidate always ranks ahead of an unverified one, and
  a delayed refine must never replace trusted Khmer with a worse alternative or raw roman.

## Consequences

- **`lexicon_verified` is load-bearing UI, not decoration.** If the `✦` marker regresses, unverified
  model output would be indistinguishable from reviewed data — that is the failure this ADR guards
  against. It must stay phrase-level accurate and survive the decode → session → adapter path.
- **The seam's inert-by-default property is a hard invariant.** `SpanProposalMode::Model` with no
  provider must resolve to `None` and never panic; the free build must compile, link, and test with
  no provider present. (Enforced by `seam_is_inert_without_a_configured_provider` and the iOS
  `smart_mode_without_provider_still_decodes` test.)
- **The provider name and weights stay out of the public repo.** Public code carries only the
  generic trait and a provider-agnostic Standard/Smart surface.
- The original offline authoring use of the model (proposing **Lexicon** rows for human review) is
  unchanged and still valid; this ADR only adds the runtime path alongside it.
