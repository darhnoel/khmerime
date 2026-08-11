# Show phrase candidates before segment candidates

Status: accepted

Windows TSF previously rendered `SessionSnapshot.segment_preview` as a complete
phrase header and `SessionSnapshot.candidates` as rows underneath it. During a
**Segmented Session**, those rows are alternatives for only the focused segment.
Putting both levels in one unlabelled popup made correct word alternatives look
like incorrect alternatives for the complete phrase.

Windows now uses a two-level **Candidate Surface**:

```text
Segmented Session              Tab: Segment Edit Mode
-----------------              ----------------------
complete Phrase Candidates  -> focused-segment Candidate List
Up/Down/Space cycle phrases     Up/Down/Space cycle words
1..9 select a phrase            1..9 select a word
```

Space also cycles the focused word while Segment Edit Mode is active. It never
commits from that mode; Enter is the explicit whole-phrase commit key. Direct
number selection likewise changes the word without committing.

Flat, single-segment composition keeps the existing Candidate List. Tab is eaten
only while a Segmented Session exists; otherwise Windows receives it normally.
Enter commits the currently previewed composition at either level.

## Deep module boundary

`render::candidate_surface::CandidateSurface` is the only Windows module that
decides which candidate level is visible. It projects a `SessionSnapshot` into
rows, display metadata, context, selection, and phrase-selection commands. The
Win32 popup paints that projection and does not inspect session modes itself.
The session driver asks the same projection whether a key selects a Phrase
Candidate, preventing render and interaction policy from drifting apart.

This is deliberately adapter policy, not new ranking logic. `khmerime_session`
still owns Phrase Candidates, Segment Edit Mode, selection state, and commits;
Windows only chooses which existing level to expose. ADR-0014 remains the mobile
Phrase Wheel decision and does not define this desktop presentation.

## Verification

- Projection tests cover Flat, Phrase, and Segment surfaces.
- A driver test types a real segmented input, cycles a whole Phrase Candidate,
  presses Tab, and verifies the surface changes to segment candidates.
- Key conversion and key-sink tests keep idle Tab passthrough and segmented Tab
  consumption explicit.
