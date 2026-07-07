# ADR-0014: Phrase Wheel replaces the default candidate row; word candidates move to per-phrase editing

**Status:** Accepted — supersedes ADR-0009 for mobile (iOS + Android)

## Context

ADR-0009 put an always-on, word-level candidate row on iOS to match Android's
permanent candidate-chip row. In use, that row as the *default* surface earns
little: for a multi-word composition the user almost never wants to browse
alternates for one segment — they want to accept (or lightly correct) the whole
phrase. Word-level candidates are only useful when the user *intends* to edit a
specific word.

Meanwhile the decoder already has what a whole-phrase chooser needs. The
weighted-span beam search (`crates/core/src/decoder/weighted_span.rs`) builds
`finals` — a ranked, truncated list of `BeamItem`s, **each a complete
whole-sentence hypothesis** carrying its own segmentation (`spans`), Khmer output
(`words`), and roman slices (`recovered_romans`). Today only `finals[0]` is
surfaced (as the Strip's Khmer Row / the **Segmented Session**); `finals[1..N]`
are computed and thrown away. Surfacing them is the feature.

## Decision

A two-level mobile candidate model, the same on iOS and Android. Each
whole-composition hypothesis is a **Phrase Candidate** (see CONTEXT.md).

**Level 1 — Phrase Wheel (default).** A horizontal, snap-to-center carousel of
Phrase Candidates. Each card pairs the roman segmentation with the concatenated
Khmer and the two scroll *together*; the centered card is the selection. It
replaces both the single-line Khmer preview and — as a default surface — the
word-level candidate row. Commit is unchanged from today's iOS behavior: **Space**
commits the centered card and appends a space; **Enter** commits it (newline only
when nothing is composing). The Roman Row shows the centered card's segmentation
and re-renders on every snap (segmentation may change between cards).

**Level 2 — per-phrase editing.** A **double-touch on the centered Khmer**
expands that card into separated, tappable words plus a word-level candidate row
for the focused word — the mobile form of **Segment Edit Mode**. Tapping a Khmer
word moves focus; tapping a word candidate settles it and stays in Level 2;
**typing re-spells the focused word** (the existing Segment Edit Mode rule);
**Space/Enter commit** the whole phrase; **double-touch** returns to the wheel
without committing. On exit the wheel re-decodes with the edited word anchored
(reusing the decoder's existing `decode_with_anchors`). Level 2 is per-phrase and
**never sticky**: any commit resets the composition and the next one starts at
Level 1.

**Locked details.**
- Two cards with the *same* Khmer but different segmentation are both kept — they
  differ by roman and the segmentation matters for editing.
- The raw-roman fallback (ADR-0013's floor) is the wheel's last card.

**Engine / FFI changes.**
- `DecodeResult` carries the top-N `finals` (not just `finals[0]`).
- `SessionResult` gains `phrase_candidates: Vec<PhraseCandidateEntry>` and
  `selected_phrase_index`; a `SelectPhrase(i)` command rebuilds the
  **Segmented Session** from `finals[i]`.
- The existing per-segment `candidates` / `segment_edit_*` fields are reused
  unchanged for Level 2.

## Consequences

- **Reverses ADR-0009's central decision** (always-on word row). The word-level
  candidate row survives only as the Level-2 editing surface.
- **Both platforms change.** iOS gains the wheel; Android's permanent chip row is
  reframed as the Level-2 surface. The two-level model is shared.
- The redundant single-line Khmer Row folds into the wheel's centered card, so
  the composing screen shows the Roman Row + the wheel, not three Khmer surfaces.
- Selecting a Phrase Candidate is a pure view/re-rank (no re-decode); only Level-2
  exit re-decodes (with an anchor). Keeps the hot path cheap.
- ADR-0009 code is reachable in git history, so reverting to the always-on row if
  the wheel doesn't land is a normal revert, not a rebuild — same escape hatch
  ADR-0009 itself relied on.

## When to revisit

- If whole-phrase selection proves rarely used and users mostly single-word, the
  wheel could collapse back toward a word-level row for short compositions.
- If the decoder starts returning many near-duplicate phrases, the "keep both
  same-Khmer cards" rule and the wheel length want a dedup/near-dup pass.
