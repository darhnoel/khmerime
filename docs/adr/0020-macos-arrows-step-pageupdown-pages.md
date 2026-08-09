# ADR-0020: macOS ↑/↓ step one candidate; PageUp/PageDown jump a page

**Status:** Accepted — supersedes ADR-0018 (macOS page navigation on ↑/↓)

## Context

ADR-0018 gave the macOS ↑/↓ arrows a whole-page jump, on the reasoning that ↑/↓ were
redundant with Space (both were `cycle_candidates`) and that "nothing is lost in practice."
It named its own revisit trigger: the paging model no longer serving the user.

That trigger arrived once the closed model shipped on macOS and the candidate list became
worth navigating. With page size 10 and up to ~20 candidates, ↑/↓ could only ever land on
row 0 or row 10 — **every word in between was unreachable by arrow.** A user reaching for
the arrows to pick a candidate found selection "not working": it jumped past the target.
Space still stepped one-at-a-time, but arrow-to-select is the stronger muscle memory.

ADR-0018's premise ("Space and ↓ were identical, so nothing is lost") held only while lists
were short. On a long model-assisted list the two behaviors are meaningfully different, and
collapsing arrows into paging removed the fine-grained selection users expect from arrows.

## Decision

- **↑ / ↓ step one candidate** on macOS (wrapping), matching Space, iOS, IBus, and TSF —
  the intuitive default. Mid-list words are reachable again.
- **PageUp / PageDown jump a whole page**, keeping the row within the page and clamping to a
  short final page — exactly the behavior ADR-0018 put on ↑/↓, just moved to the page keys.
- **The translation stays in the macOS adapter** (`handle_event` recognises PageUp/PageDown
  keysyms `0xFF55`/`0xFF56` from `KeyvalMapping`), driving the session's own one-step cycling
  to reach the target index. The shared session is untouched, so IBus and TSF are unaffected.
- Left/Right remain segment navigation; digits `1`–`9`/`0` still pick within the visible page.

## Consequences

- The macOS Candidate List stays fully traversable: ↑/↓ walk entries, PageUp/PageDown walk
  pages, digits pick within a page.
- Fast tail-reaching (ADR-0018's goal) is preserved — it moved from arrows to the page keys,
  where it does not collide with per-item selection.
- `page_jump_target` is unchanged; only its trigger key changed. The adapter still owns the
  page math, per ADR-0018's "keep the other platforms frozen" constraint.
