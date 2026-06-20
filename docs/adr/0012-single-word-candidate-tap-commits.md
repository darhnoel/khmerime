# ADR-0012: Tapping the preedit word commits a single word; a phrase focuses the segment

**Status:** Accepted

## Context

The preedit's Khmer row shows the chosen Khmer for the composition: the selected
candidate for a single word, or one chip per segment for a multi-word phrase. For a
single word the label was inert (`setOnClickListener(null)`), so even though the
chosen word sat right there under the roman, committing it required Enter. iOS already
commits a single word straight from this row in `chipTapped` ("a single word has no
segments — tapping it commits directly").

The Suggestion Bar is a different surface: it lists the candidate alternatives so the
user can pick a spelling.

## Decision

| Preedit Khmer-row tap | Single word (no segments) | Multi-word (segments) |
|---|---|---|
| effect | **Commit the shown selected candidate** | **Focus that segment** (its candidates appear in the Suggestion Bar) |

The **Suggestion Bar stays select-only**: tapping a candidate there picks a spelling
and updates the preedit word **without committing**, so the user can choose a
non-default spelling and *then* tap the word to commit. Android routes the preedit tap
through `focusSegment`, which commits when there are no segments — the same unified
Khmer-row tap handler as iOS `chipTapped`.

## Considered Options

- **Commit on a Suggestion Bar tap for single words** (the initial, reverted
  implementation) — rejected: the Suggestion Bar is the surface for *choosing* among
  spellings, so committing there removes the ability to pick a different candidate
  before committing.

## Consequences

- To commit a non-default spelling: pick it in the Suggestion Bar (updates the preedit
  word), then tap the word.
- `focusSegment` gains a no-segments commit branch, and the single-word preedit label
  becomes tappable; multi-word segment focusing is unchanged.
