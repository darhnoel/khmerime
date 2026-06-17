# ADR-0009: iOS persistent candidate row replaces panel browsing; panel becomes CharPick-only

**Status:** Accepted

## Context

Android never swaps its qwerty keys out for a separate panel: a candidate-chip row sits permanently above the keys (`KeyboardPresentationSpec.suggestionCandidates` returns `state.candidates` unconditionally, `renderStateReplacesKeyboardLayer` is hardcoded `false`). iOS instead used `CandidatePanelView`, a full replacement of the key rows, opened via ✦ or strip long-press, to browse and select among multiple candidates and to enter Segment Edit Mode via a ✏ button on each chip.

While investigating this gap we found ADR-0008 itself was inaccurate: it states "tapping any candidate in the panel commits that variant immediately and closes the panel," but no code path does that — `selectCandidate(at:)`'s non-charPick branch only updates `selected_index` (confirmed in Rust by the test `digit_selects_candidate_without_immediate_commit`). Tap-to-commit only ever existed for the single-word case on the segment row (`chipTapped`'s early return when there are no segments), which ADR-0008 also documents and which remains correct.

## Decision

- Add a persistent candidate-chip row between the strip and the key views, always visible, tap = select only (never auto-commits — matches the real `selectCandidate` behavior and Android's digit-select semantics). Grow `baseKeyboardHeight` by ~44pt (260→304 phone, 320→364 pad) to make room without shrinking key touch targets below 44pt.
- The segment row gains Android's 2-tap-to-edit gesture: tapping an already-focused segment enters Segment Edit Mode inline via `session.sendTab()`, reusing the bracket-highlighted rendering that already exists in `StripPresentationSpec.editModeText` (built for the panel's ✏ button, now driven by this gesture instead). Tapping a *different* segment just moves focus — no panel involved either way.
- `CandidatePanelView` keeps only its CharPick rendering path (`renderCharPickAlphabet` / `renderCharPickCandidates`); the chip-row/candidate-grid/✏-button browsing machinery (`rebuildChips`, `makeChipContainer`, `editTapped`, `didTapChipAt`, `didRequestEditAt`) is deleted as dead code.
- ✦ (and strip long-press) now unconditionally enters CharPick, matching Android's `toggleSuggestCharacter`, instead of branching on whether a composition exists.
- The new candidate row clears whenever CharPick is active, the same way the strip already clears (`onStripClear?()`), to avoid two redundant candidate UIs on screen at once.
- ADR-0008's "panel candidate tap commits + closes" line is corrected to match the actual, unchanged behavior: tap selects, Enter commits.

## Consequences

- Built on its own branch with the prior panel-browsing code left intact in git history (not force-deleted in a way that's hard to find) — reverting to the old panel-based browsing model, if the new row doesn't feel right in practice, is a normal `git revert` rather than a rebuild.
- The keyboard extension is taller in every state (qwerty/numeric/symbols/panel), not just while candidates are showing — less visible app content behind the keyboard, most noticeable on smaller phones.
- CharPick's full-screen panel UI is unchanged; only its sibling "browse multi-word candidates" role moves out of the panel entirely.
