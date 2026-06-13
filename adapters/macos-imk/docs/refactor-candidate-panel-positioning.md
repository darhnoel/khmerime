# Refactor: macOS candidate panel positioning

## Problem Statement

The macOS candidate panel does not reliably appear under the text caret. The
positioning code has been patched four times and still fails:

1. `firstRect(forCharacterRange: {NSNotFound, NSNotFound})` → zero rect → panel
   stranded in a screen corner.
2. Switched to `{0, 0}` → panel anchored at the composition *start*, so it never
   followed the caret while typing.
3. Switched to `{preedit.length, 0}` (end of marked text) → **current bug**: that
   index is *one past* the last marked glyph, and clients answer an out-of-range
   marked-text index with a degenerate rect at the line/document origin. Result:
   the panel pins to the **left margin** instead of sitting under the caret
   (reproduced in TextEdit and Notes with real committed text on screen).

Each fix addressed a symptom of one observed case without a model of what
`firstRectForCharacterRange:` actually returns, so the next case re-broke it. The
underlying issue is that the **caret-rect derivation** at the IMK boundary is
guessed, untested, and not grounded in real client data.

Separately, the candidate list was changed from a horizontal "candidates row"
(documented in ADR-0003 as iOS two-row parity) to a vertical IBus-style column,
so the code now contradicts its own ADR.

## Solution

Anchor the panel at the **trailing edge of the last marked glyph**: query
`firstRect(forCharacterRange: {length-1, 1})` (a range that stays *inside* the
valid marked text) and use the returned rect's trailing edge as the caret point.
This follows the caret as the composition grows without ever querying an
out-of-range index. Empty preedit falls back to the `{0, 0}` insertion point.

Crucially, validate the exact range/edge choice against **captured ground-truth
`firstRect` values** before committing the behavior change, instead of guessing
again. Keep the existing, well-tested pure `CandidatePanelLayout` geometry (below
/ flip / clamp / line-height floor) untouched — only the caret-rect *input* to it
changes, and that derivation becomes a pure, tested function.

Finally, reconcile ADR-0003 with the now-vertical candidate list.

## Commits

A sequence of tiny commits, each leaving the build green.

1. **Capture ground truth (temporary instrumentation).** Extend the existing
   `[DEBUG-macos-imk-runtime]` geometry log so that, for each panel update, it
   logs the rects returned for several candidate ranges of the marked text —
   `{0,0}`, `{0, length}`, `{length-1, 1}`, and `{length, 0}` — together with the
   `actualRange` out-parameter each one reports. Reinstall, then type in both
   TextEdit and Notes at: start of an empty document, mid-line, end of a long
   line, and a wrapped line with committed text above. Collect the logs. This
   commit changes only logging; behavior is unchanged. The data decides whether
   `{length-1, 1}` is the right range in every client or whether the fallback
   (below) is needed.

2. **Add a pure `caretAnchorRange(preedit:)` function.** Returns the marked-text
   range to query: `{length-1, 1}` for a non-empty preedit (UTF-16 units, so a
   multi-scalar Khmer cluster counts correctly) and `{0, 0}` for an empty
   preedit. Cover it with unit tests. Not wired in yet — the old path still runs,
   so the build stays green. Replaces the role of the current
   `caretQueryRange(preedit:)`.

3. **Add a pure `caretPoint(fromGlyphRect:)` (trailing-edge) function.** Given the
   rect of the last marked glyph, return a zero-width caret rect positioned at the
   glyph's **trailing edge** (`maxX`), preserving the glyph's vertical extent, so
   the existing `CandidatePanelLayout.origin(caret:…)` places the panel at the end
   of the composition. Unit-tested. Still not wired in.

4. **Wire the boundary to the new derivation.** In the controller's panel-update
   callback, query `firstRect` with `caretAnchorRange(preedit:)`, transform the
   result through `caretPoint(fromGlyphRect:)` when the preedit is non-empty (use
   the rect as-is for the empty/insertion-point fallback), and pass it to
   `panel.show`. Remove the now-unused `caretQueryRange`. Behavior change; verified
   manually next.

5. **Manual verification against ground truth.** Reinstall and confirm the panel
   sits under the caret at: start, mid-line, end-of-line, and wrapped-line
   positions, in both TextEdit and Notes — and that the typed line stays visible
   above the panel. No code change; this is the gate before cleanup.

6. **Remove the temporary geometry logging.** Strip all `[DEBUG-macos-imk-runtime]`
   instrumentation added in commit 1 (and any left from earlier rounds).

7. **Reconcile ADR-0003.** Update (or supersede) the custom-NSPanel ADR to record
   that the macOS candidate list is a **vertical** IBus-style column while the
   segment chips row remains horizontal, and why the divergence from the iOS
   two-row layout is deliberate (desktop vertical space; wide Roman Hints read
   better one-per-row).

## Decision Document

- **Modules modified:**
  - The pure panel-layout module gains `caretAnchorRange(preedit:)` and
    `caretPoint(fromGlyphRect:)`; its existing `origin(caret:panelSize:screen:)`
    geometry is unchanged.
  - The IMK input-controller boundary changes only *which* marked-text range it
    queries and how it interprets the returned rect.
  - The candidate panel's `show(below:)` interface is unchanged.
- **Anchoring contract:** marked-text character indices are relative to the
  marked text; the code must never query at `location == length` (out of range).
  The caret is taken as the **trailing edge of the last marked glyph**; an empty
  preedit uses the `{0,0}` insertion-point rect.
- **Ground-truth gate:** the exact range/edge is confirmed from captured real
  `firstRect` values before the behavior change ships. If `{length-1, 1}` proves
  unreliable in some client, fall back to `{0, length}` and use that rect's
  trailing edge.
- **Architecture preserved:** custom `NSPanel` + `firstRectForCharacterRange:`
  stays (per ADR-0003's segment-chip requirement). `attributesForCharacterIndex:`
  was considered and deferred — only revisited if ground truth shows `firstRect`
  is unusable.
- **Docs:** ADR-0003 updated to reflect the vertical candidate list.

## Testing Decisions

- **What makes a good test here:** assert external behavior of the pure
  functions — given a preedit, the right query range; given a glyph rect, the
  right trailing-edge caret point; given a caret rect, the right on-screen origin.
  Do **not** assert AppKit/IMK internals.
- **Modules tested:** the pure panel-layout module (`caretAnchorRange`,
  `caretPoint`, and the existing `origin` geometry).
- **Not unit-tested:** the `firstRect` call itself (needs a live IMK client) — it
  is verified through the ground-truth capture and the manual reinstall pass,
  consistent with how the runtime key path has been verified throughout.
- **Prior art:** `CandidatePanelLayoutTests` (geometry cases) and
  `SessionKeyInputTests` (boundary-input derivation from synthesized events).

## Out of Scope

- IBus-style **paging** of long candidate lists (the taller-than-screen clamp
  stays as the safety net for now).
- Candidate-row **styling / typography / Roman Hint hierarchy**.
- Switching the anchoring API to `attributesForCharacterIndex:lineHeightRectangle:`
  (deferred unless ground truth forces it).
- Any **iOS** candidate-panel changes.

## Further Notes

- The pure `CandidatePanelLayout.origin` geometry (below-with-floored-line-height,
  flip-above, top-biased vertical clamp, horizontal clamp) is already test-covered
  and is **not** the source of this bug — the bug is upstream, in the caret rect
  fed into it. This refactor deliberately leaves that geometry alone.
- "Chase the caret per keystroke" is a settled product decision; the trailing-edge
  anchor is what makes chasing land on the actual caret rather than the left
  margin.
