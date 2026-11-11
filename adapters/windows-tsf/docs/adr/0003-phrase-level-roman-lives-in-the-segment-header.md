# Phrase-level roman lives in the segment header (macOS parity)

Status: accepted

Supersedes the per-row phrase roman from commits `2c5f306` and `08cbae7`.

On the phrase level of the two-level Candidate Surface (ADR-0002), Windows showed the
roman twice: a context line `ខ្ញុំ(knhom) | [ទៅ(tov)] | ផ្សារ(phsar)`, and again on every phrase
row (`knhom tov phsar`, `knhom tov phsa`, …). Because each segment's canonical spelling ranks
the exact input first, the rows mostly echoed what the user had just typed, and the header
echoed it a second time.

Windows now follows the macOS panel (macos-imk ADR-0004):

```text
Phrase level (dim context header)     Word level, after Tab (focus highlighted)
  ខ្ញុំ     ទៅ    ផ្សារ                       ខ្ញុំ   [ទៅ]   ផ្សារ
 knhom   tov   phsar                    knhom  [tov]  phsar
 ─────────────────────                  ─────────────────────
 1   ខ្ញុំទៅផ្សារ                           1   ទៅ
 2   ខ្ញុំទៅផ្សា                            2   ទូ
 3 ✦ ខ្ញុំទុកផ្សារ
```

- The header is two aligned rows: each segment's Khmer, with the roman typed for it
  (`SegmentPreviewEntry.input`) beneath, one column per segment.
- On the phrase level the header is dimmed context and never looks selected. On the word level
  the segment being edited is highlighted, so word alternatives cannot be mistaken for phrase
  alternatives (the ADR-0002 problem).
- Phrase and word rows are Khmer-only. They keep the ✦ model mark (provenance is about the
  reading, not its roman), but drop ✓ and ~, which describe a roman the row no longer shows.
- The flat Candidate List has no header, so its rows keep their Roman Hints and marks.

Linux IBus follows the same rule within its single-line aux text (linux-ibus ADR-0001, amended the
same day): phrase rows are Khmer-only while the segment preview is shown.

The session still carries per-segment Roman Hints on `PhraseCandidate`; this is a display
decision in `render::candidate_popup`, not a data change. The test-only inline preview keeps its
old labels.

## Trade-off

A phrase row whose reading spells a segment differently from the input (row 2's `phsa` vs the
typed `phsar`) no longer says so on the row; the user reads the Khmer instead. We accept this for
parity with macOS and a quieter popup. If that difference turns out to matter, the follow-up is
to show a row's roman only where it differs from the header, not to restore it on every row.
