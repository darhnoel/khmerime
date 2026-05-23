# Segment Edit Mode — edit one segment of a Segmented Session in isolation

Within a [Segmented Session](../../CONTEXT.md), the user can now press **Tab** on the focused segment to enter **Segment Edit Mode** — a sub-state in which that segment's roman is rewritten in isolation while sibling segments stay pinned with their currently selected Khmer outputs. Tab toggles in and out; Escape cancels and restores; Enter and Space commit the whole composition. Digit keys 1–9 select the Nth candidate for the in-edit segment without committing, so Enter remains the explicit whole-composition commit gesture. The in-edit segment is rendered in the inline **Preedit** with both an underline and a background highlight; the auxiliary popup keeps the existing `⟦…⟧` brackets around the focused segment.

We chose this design after walking the decision tree across thirteen branches; the key trade-offs worth remembering are below. Most of these decisions are individually small but their *combination* is the contract — changing any one in isolation will break the others.

## The key decisions

- **Focus alone does not arm replacement.** Left/Right move focus and only draw an underline. Tab is the explicit, separate gesture that promotes focus into edit mode. The two-step UX exists so that users browsing through segments (a common inspection pattern) cannot accidentally destroy a segment with a stray keystroke. The cost — one extra key for the "I want to fix segment 3" path — is acceptable because most edits are deliberate.

- **Siblings are pinned; the decoder does not touch them.** Editing one segment does not re-run global segmentation over the whole roman string. Pinned siblings keep their Khmer choices verbatim. This honours the user's stated intent ("fix *this* chunk, leave the rest alone") and avoids the gnarly UX of segment boundaries silently shifting under the user. The escape hatch for "I actually want global re-segmentation" is the existing Escape-the-whole-session flow.

- **The in-edit segment runs the flat decoder, not the segmenter.** If the user types something the segmenter would normally split into two words, we *do not* split it — we present one candidate list for the whole slice. Segment count is invariant during Segment Edit Mode. Sub-splitting would require the auxiliary popup, span-tracking, and `focused_segment_index` accounting to all handle mid-edit mutation, which we deemed not worth the marginal benefit.

- **Keystroke semantics are intentionally asymmetric.** The first printable key after Tab replaces the entire roman slice (text-editor-style "selection replace"). Backspace deletes one character at a time (IME-style). The background highlight is an edit-mode *indicator*, not a literal text-editor selection. The asymmetry is hidden because users intend one of two operations at a time — "redo this completely" (types) or "nudge this" (backspaces) — never both. The alternative (full text-editor semantics, where first Backspace clears the whole segment) was rejected as a 50/50 footgun.

- **Backspace on an empty in-edit segment transfers the mode to the previous segment** rather than leaving a phantom zero-width segment behind. If the in-edit segment was the first one, Backspace is a no-op. If exactly one segment remains after collapse, the **Segmented Session** dissolves to a flat **Composition** and Segment Edit Mode ends. This matches text-editor backspace-across-line-boundary behaviour and keeps the **Composer** invariants free of empty-segment edge cases.

- **Left/Right auto-exit edit mode and navigate.** They do *not* move a caret inside the in-edit segment's roman. We deliberately did not add a per-segment caret position to the **Composer** state — the replace-on-first-keystroke and backspace-from-end model makes mid-string caret movement unnecessary, and the missing data would be a big new surface area for the wins of "fix one char without retyping."

- **Tab is inert outside a Segmented Session** and falls through to the host application when there is no **Composition** at all. The Tab gesture has no meaning in a flat **Composition** — there is nothing to "edit into." Forcing a single-segment session to make Tab universally meaningful would violate the **Segmented Session** invariant of having internal word boundaries.

- **Enter and Space commit the whole composition; digits only select.** Enter remains the explicit "ship it" gesture, and Space keeps the existing IME-wide commit shortcut. Digit keys 1–9 select the Nth candidate for the in-edit segment and keep Segment Edit Mode active. We rejected digit-implies-commit because choosing among segment candidates is an inspection/editing action, and committing the whole **Composition** from that same key is too easy to trigger prematurely.

- **Zero-candidate edits commit literal roman.** If the in-edit roman has no Khmer match, the segment behaves like a flat **Composition** with no match — the raw roman itself is the candidate. We considered blocking exit until at least one candidate exists, but a silent block with no error UI is hostile, and silently discarding the user's input on commit is worse.

## What this constrains

The combination of these decisions implies that:

1. The **Composer** must support a "pinned segment" data shape — a segment whose Khmer is locked and not re-decoded — that is currently absent.
2. The decoder needs a knob to force single-segment output (the in-edit segment runs flat). `crates/core/src/decoder/config.rs` does not yet expose this; it will need a new flag (e.g. `max_segments: 1` or `flat: true`).
3. The IBus preedit attribute layer in `khmerime_ibus_engine.py` needs to emit both an underline and a background colour on the same span when Segment Edit Mode is active, rather than only the underline that exists today.
4. The bridge snapshot schema needs a new field (e.g. `segment_edit_active: bool`) so the Python adapter can distinguish "focused" from "focused-and-in-edit-mode" — both are presently collapsed under `focused_segment_index`.

## When to revisit

This decision should be reconsidered when **any** of the following is true:

- Users repeatedly ask to edit individual characters mid-segment (i.e. demand a per-segment caret). The Left/Right→navigate decision would flip and the **Composer** would need caret state.
- We add a mouse-driven popup where clicking a segment should plausibly enter edit mode directly; the Tab-only entry gesture may then feel impoverished.
- The decoder gains incremental re-segmentation that is cheap enough to run on every keystroke; the "pinned siblings, flat in-edit decode" model could relax into "always re-segment globally, but show the original boundaries by default."
