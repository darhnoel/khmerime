# Explicit spelling review overlays the Document

Status: accepted

## Context

Khmer spelling candidates need to inspect overlapping token spans because a
single best segmentation can hide a nearby longer word. For example,
`សាលារាន` may segment as `សាលា | រាន`, while `សាលារៀន` remains a useful
one-edit alternative. Validation also showed that segmentation plus edit
distance is advisory rather than authoritative: even conservative filtering
raised candidates in 37.5% of reviewed correct sentences.

ADR-0001 keeps the Document as a plain `textarea` to protect Khmer Composition.
Inline spelling marks must not replace that input surface or mutate text without
an explicit user choice.

## Decision

- Spelling review runs only from the sidebar action
  **ពិនិត្យអក្ខរាវិរុទ្ធ**. It never runs while the user types.
- Results are possible alternatives, shown with an amber dotted underline. They
  are not labelled errors and never replace text automatically.
- A synchronized, read-only highlight layer temporarily overlays the textarea.
  The textarea remains the document value, selection, caret, and input owner.
- At most five non-overlapping matches are shown. A temporary result bar moves
  between them, and clicking a mark opens an anchored correction popover with
  the best-ranked alternative first.
- The result bar is a compact neutral review toolbar; amber is reserved for the
  advisory underline. A clean result is confirmed briefly and then dismissed.
- The correction popover stays compact and anchored to its mark, but measures
  available space and shifts or flips above the mark to remain inside the
  Document. The selected correction is the primary action; other alternatives
  and ignore remain visually secondary.
- Manual editing clears the complete review because stored character spans are
  then stale. Accepting one correction adjusts later spans; ignoring removes
  only that match. Neither action automatically reruns review.
- Saved Khmer words are valid spellings and participate in correction lookup.
- `khmer-tokenizer-core` supplies segmentation hints and `symspell_rs` supplies
  one-edit candidates. Overlapping windows may contain unknown fragments because
  the misspelling itself can break an otherwise valid word into such fragments.
  Ranking prefers smaller edit distance, the closest source/candidate length, a
  longer candidate, fewer joined tokens, and then dictionary frequency.

## Consequences

- Existing Composition and candidate-surface behavior stays on the proven
  textarea path.
- The overlay must exactly share the textarea's padding, font, line height, and
  wrapping rules; browser regression coverage protects this interaction.
- The feature may still show unwanted alternatives. Its language, color, and
  explicit invocation communicate uncertainty until contextual ranking and a
  human-labelled Khmer typo corpus improve precision.
