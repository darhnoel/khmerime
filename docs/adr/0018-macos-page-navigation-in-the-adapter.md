# ADR-0018: macOS gets Up/Down page navigation, implemented in the adapter

**Status:** Accepted — supersedes ADR-0013's "no explicit page keys" decision **for the macOS IMK only**

## Context

ADR-0013 decided pagination would emerge from Space plus cursor movement, with no
page-jump key, and listed the condition for revisiting it:

> *Users demand direct page navigation (Page Up/Down or clicking a page). The
> "pagination emerges from Space + cursor movement" decision would flip toward
> explicit page keys.*

That condition arrived. With the macOS panel now painting a page at a time (page size
10) and the **Candidate List** holding up to 20 plus the raw roman fallback, reaching
the tail of the list costs up to 20 Space presses. The raw fallback — the one entry a
user reaches for when the Khmer is wrong — is the furthest away.

Two facts made the fix cheap:

1. **Up/Down were redundant.** `handle_up` / `handle_down` in the shared session are
   literally `cycle_candidates(-1)` / `cycle_candidates(1)` — the same thing Space
   already does. Two keys doing one job, while page-jumping had none.
2. **Space already wraps.** `offset_index` uses `rem_euclid`, so cycling loops the list.
   Page jumping can wrap the same way without inventing a new selection model.

The constraint that shaped the design: **Linux (IBus) and Windows (TSF) behavior must
not change.** They share `crates/session`, so moving page logic into `handle_up` /
`handle_down` there would silently alter both — platforms the change was never
requested for and was not tested against.

## Decision

- **↑ / ↓ jump a whole page** in the macOS candidate list; **Space is unchanged**
  (one candidate at a time, wrapping, flipping the page as the selection crosses a
  boundary). Left/Right remain segment navigation.
- **Jumping wraps**, consistent with Space: ↓ past the last page returns to the first,
  ↑ from the first goes to the last.
- **The jump preserves the row within the page** where possible, clamped to the length
  of the destination page — so ↓ from row 3 lands on row 3 of the next page, and a
  short final page (e.g. the lone roman fallback) clamps to its last row.
- **The translation lives in the macOS adapter**, not the shared session. `handle_event`
  recognises ↑/↓ during an active **Composition** and drives the session's existing,
  already-tested candidate cycling to reach the target index. The shared
  `handle_up` / `handle_down` keep their current one-step behavior, so IBus and TSF are
  byte-for-byte unchanged.

## Consequences

- The macOS **Candidate List** is fully traversable: ↓ walks pages, Space walks entries,
  digits `1`–`9` and `0` select within the visible page.
- ADR-0013's "no page keys" reasoning still holds for the adapters that did not opt into
  paging; this ADR narrows it rather than replacing it. An adapter that later wants page
  keys should decide deliberately, not inherit them.
- The cost of keeping the other platforms frozen is that page navigation is adapter-level
  rather than shared. If TSF or IBus later adopt page keys, the logic should be promoted
  into the session (behind the `page_size` gate) instead of being copied a third time —
  that promotion is the point at which this ADR is revisited.
- ↑/↓ lose their one-step meaning on macOS. Nothing is lost in practice: Space and ↓ were
  identical, and Space keeps the fine-grained behavior.
