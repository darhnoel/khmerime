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

- **Source.** `phrase_candidates` reads the **weighted-span (WFST)** result as
  the primary source — the same decoder that feeds the strip — then appends
  segmented legacy/composer phrase combinations that are useful Khmer
  whole-phrase candidates. This keeps WFST ranking as the anchor while restoring
  alternatives such as `ខ្ញុំទៅសាលារៀន`, `ខ្ញុំទៅ៏សាលារៀន`, ... for multi-word
  inputs like `nhomttovsalarien`. Legacy raw-roman fallback remains excluded
  from the wheel.
- **Contents — alternatives only.** The wheel shows the hypotheses *other than* the
  currently selected one; the **Strip**'s Khmer Row already shows that selected
  phrase. No raw-roman card. (The raw string remains the **Commit Rules** floor,
  just not a wheel card.)
- **Visibility.** Shown only when at least one visible alternative exists after
  excluding `selected_phrase_index`; otherwise the strip stands alone and the
  candidate-row height collapses.
- **Layout — balanced.** Cards are centered when they all fit the width, and
  left-padded + horizontally scrollable when they overflow — reusing
  `CandidateRowLayout.centeringInset`.
- **Selection — tap selects.** Tapping a card makes that Phrase Candidate the
  strip preview by calling `select_phrase(index)` and setting
  `selected_phrase_index`. It does not commit. Space/Enter commit the selected
  reading (the strip's preview). The center-snap "alarm-clock" selection from
  ADR-0014 is retired.

## Consequences

- Reverses ADR-0014's snap-to-center selection, "show all hypotheses (+ raw, roman
  pairing)" contents, and always-on visibility.
- The engine keeps exposing the **full** ranked list (`phrase_candidates` =
  `finals[0..N]` plus useful segmented phrase combinations); "alternatives only"
  is a UI projection that excludes `selected_phrase_index`, so the data model
  stays general and selection is reversible.
- Simplifies the iOS view: the snap/scroll-settle machinery is deleted in favour
  of a tap-select centered/scrollable row.
- The wheel's word-level editing entry is unchanged (strip chips → Segment Edit).
- iOS chrome has a `stripOnly` state: normal composition keeps the strip visible
  but reserves the candidate-row height only when the wheel has visible
  alternatives.

## When to revisit

- If WFST routinely returns many near-duplicate readings, "alternatives only" plus a
  dedup/near-dup pass (the deferred keep-both/dedup work) want revisiting together.
- If users miss seeing the committed phrase inside the wheel, reconsider "alternatives
  only" vs showing the best as a pinned first card.
