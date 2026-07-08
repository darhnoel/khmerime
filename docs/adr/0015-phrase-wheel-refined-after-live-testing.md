# ADR-0015: Phrase Wheel refined after live iOS testing

**Status:** Accepted — supersedes parts of ADR-0014 (the Phrase Wheel's source, contents, visibility, layout, and selection)

## Context

Running the ADR-0014 wheel on a device surfaced a bug and sharpened the UX. Typing
a long phrase (`derlengjeamouymitpheak`) showed the *raw roman* as the wheel's card
while the strip correctly showed the Khmer segmentation (ដើរ លេង ជា មួយ …).

Root cause: the iOS session uses `DecoderConfig::shadow_interactive()` (mode
`Shadow`), and `phrase_candidates` was built on `choose_visible_result`, which for
`Shadow` mode returns the **legacy** decoder. Legacy can't produce a whole-phrase
reading for a long input, so it falls back to the raw roman. Meanwhile the strip's
`segment_preview` reads the **weighted-span (WFST)** top path, which segments
correctly. The two surfaces were reading different decoders.

## Decision

Refine the Phrase Wheel (both platforms):

- **Source.** `phrase_candidates` reads the **weighted-span (WFST)** result — the
  same decoder that feeds the strip — falling back to legacy only when WFST is
  empty/failed. This is the bug fix: the wheel now shows Khmer whole-phrase
  hypotheses, and its ranking matches the strip's best.
- **Contents — alternatives only.** The wheel shows the hypotheses *other than* the
  top-ranked one; the **Strip**'s Khmer Row already shows the best. No raw-roman
  card. (The raw string remains the **Commit Rules** floor, just not a wheel card.)
- **Visibility.** Shown only when at least one alternative exists; otherwise the
  strip stands alone and the wheel is hidden.
- **Layout — balanced.** Cards are centered when they all fit the width, and
  left-padded + horizontally scrollable when they overflow — reusing
  `CandidateRowLayout.centeringInset`.
- **Selection — tap commits.** Tapping a card commits that phrase immediately
  (matching ADR-0012 and the strip's tap-to-commit); Space/Enter commit the
  top-ranked reading (the strip's preview). The center-snap "alarm-clock" selection
  from ADR-0014 is retired.

## Consequences

- Reverses ADR-0014's snap-to-center selection, "show all hypotheses (+ raw, roman
  pairing)" contents, and always-on visibility.
- The engine keeps exposing the **full** ranked list (`phrase_candidates` =
  `finals[0..N]`); "alternatives only" is a UI projection (drop index 0), so the
  data model stays general.
- Simplifies the iOS view: the snap/scroll-settle/`selectPhrase`-then-Enter
  machinery is deleted in favour of a tap-to-commit centered/scrollable row.
- The wheel's word-level editing entry is unchanged (strip chips → Segment Edit).

## When to revisit

- If WFST routinely returns many near-duplicate readings, "alternatives only" plus a
  dedup/near-dup pass (the deferred keep-both/dedup work) want revisiting together.
- If users miss seeing the committed phrase inside the wheel, reconsider "alternatives
  only" vs showing the best as a pinned first card.
