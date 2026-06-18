# ADR-0010: Silk Veil keeps glass off text surfaces on the Online Beta

**Status:** Accepted

## Context

The **Silk Veil** glassmorphism identity was designed for the **Download Landing Page** — a read-only marketing surface where translucent panes sit behind decorative content. The **Online Beta** (the dioxus-app) now reuses that identity (see CONTEXT.md), but it is a *functional typing tool*: a live composition/preedit, dense candidate/suggestion lists, segment chips, and keycaps — text the user actively reads and selects.

`backdrop-filter` glass composites whatever is behind it under the text, which is unpredictable and erodes contrast. On the Online Beta's core task — reading Khmer candidates — that is unacceptable. The app's existing theme is effectively *mono-accent* (ink + terracotta `rgba(177,77,30,·)` at ~20 opacities, no semantic hues), so porting the palette is mechanical; the real decision was how far to push translucency.

## Decision

- Apply glass (backdrop-filter blur, translucent fills, white rim highlights) **only to chrome/containers**: the shell/body, toolbar, editor-card container, guide cards, and the suggestion-popup container.
- **Never** place glass behind text the user reads or selects. The composition/preedit text, candidate/suggestion items, segment-chip outputs, and debug values sit on **solid, opaque** backgrounds.
- Map the existing terracotta accent to **ember-amber** across the same opacity ramp (state distinctions — active / recommended / hover / partial — carry over for free); **teal** stays a sparse accent. No new semantic colors are introduced.
- The debug "shadow" panel inherits the new tokens for legibility but is **not** a glass target.
- Enforce the legibility rule in CI: Playwright reads computed styles and asserts the active `.suggestion-word` and the composition/preedit text meet **WCAG AA ≥ 4.5:1** contrast against their (solid) backgrounds, plus a dark-base check and a glass-present-on-chrome check.
- TDD the theme against a fast **static harness** (real `assets/main.css` + representative markup, no WASM build); the existing full-app Playwright suite is the final integration gate.

## Consequences

- The candidate list and preedit are intentionally *not* glassy while everything around them is — by design, not an oversight.
- The contrast assertions double as a regression guard: any future restyle that drops text onto glass, or reverts to the light cream theme, fails CI.
- Reverting just the palette is a tokens-file change; the glass/legibility boundary is the costly-to-undo part because it shapes both markup layering and the tests — hence this record.
- The same rule governs both Silk Veil surfaces, keeping the Download Landing Page and the Online Beta visually coherent without copying the landing page's decorative, text-light glass wholesale.
