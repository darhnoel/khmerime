# Design handoff — dioxus editor webapp

> **Update (2026-08-23):** The layout and theme direction described below has
> been superseded by [ADR-0001](adr/0001-local-first-focused-document-workspace.md).
> The implemented first slice is now a focused document workspace with a
> responsive sidebar, centered document surface, mutually exclusive candidate
> surfaces, persistent mode-aware hints, and Light/Dark/System themes. The
> session notes below remain useful history for the original prediction bug.

> **Candidate refinement (2026-08-23):** Segmented transliteration now follows
> the macOS IMK adapter's phrase-first, two-level Candidate Surface. Normal
> typing shows whole Phrase Candidates while the Roman composition stays in the
> textarea. Tab (or the phrase-row pencil on touch) deliberately opens Segment
> Edit; Left/Right changes the focused segment; Tab/back returns to phrases.
> The old automatic segmented replacement overlay is forbidden. The web panel
> uses five-row pages, a fixed minimum footprint, caret anchoring on desktop and
> mobile, an IMK-like boundary/selection treatment, and a quiet page count.
> Manual mode no longer renders the detached workspace preview. After the first
> build/skip, its Built/Remaining/Next state appears as a compact header inside
> the content-sized caret popup; before state diverges, that header is hidden.

> **Rules sheet refinement (2026-08-23):** `ក្បួន និងផ្លូវកាត់` now opens a
> non-modal fixed sheet over the right side instead of participating in the
> Document layout. Opening it must not resize the editor. It is 400 px on
> desktop, full-width on mobile, closes through × / Escape / the active sidebar
> item, and groups the editor's real shortcuts by Normal, Phrase/Segment Edit,
> and Manual modes before the Romanization references. The sheet scrolls
> internally with its native scrollbar hidden and a bottom fade/cue.

Context for continuing the editor UI/UX work. This session redesigned the
next-word suggestion UX, re-themed the editor, and fixed a stylesheet-loading
bug. What's here is **uncommitted working state** — verify with `git diff`
before building on it.

## Run it

```bash
cd apps/dioxus-app
~/.cargo/bin/dx serve --platform web --port 8080     # then open http://localhost:8080
```

**Gotcha:** `/opt/homebrew/bin/dx` is a Deno alias that shadows the real Dioxus
CLI — always use the full path `~/.cargo/bin/dx`. A harmless
`dx 0.7.10 vs dioxus 0.7.4 incompatible` warning prints; it still serves.

## What changed this session (all uncommitted)

### 1. Stylesheet loading — a real bug fix ([main.rs](../src/main.rs))
The CSS was switched (commit 18f19fb) from embedded `include_str!` to
`document::Stylesheet { href: "/assets/main.css" }`. **`dx serve` does not
statically serve `asset_dir` files** — every `/assets/*` request falls through
to the SPA `index.html`, so NO stylesheet loaded (unstyled UI: doubled toolbar
labels, run-together hint text). Fixed by embedding the ordered CSS partials via
`include_str!` again (`APP_CSS`, injected inline). The `assets/css/*.css`
partials stay split for maintainability; they're concatenated at compile time.
Keep the `include_str!` list in `main.rs` in sync with the `@import`s in
`assets/main.css` if partials are added/removed.

### 2. Next-word predictions → docked bar ([editor_card.rs](../src/ui/components/editor_card.rs))
Design decision (from a `/design` canvas the user picked): predictions live in a
**docked bar below the editor**, not the caret popup. Implemented:
- `is_next_word = state.candidate_mode() == CandidateMode::NextWord`.
- Caret popup (`suggestion-popup`) now shows **transliteration candidates only**
  (`has_suggestions && !is_next_word`).
- New **`.next-word-dock`** strip (testid `next-word-dock`): label `បន្ទាប់` + an
  arrow SVG, pill chips (top chip accent-tinted), hint `ប៉ះដើម្បីប្រើ` ("tap to
  use"). Renders on desktop AND mobile — importantly NOT the existing
  `candidate-track-mobile`, which is `display:none` on desktop (that was the bug
  where predictions never showed).
- **Pointer-only:** `has_live_suggestions` now excludes NextWord mode, so
  Tab/Enter/Space don't act on predictions — **Enter stays free for newlines**
  (the user hit "Enter can't newline" because it was committing predictions).
  Accept a prediction by clicking a chip (`click_candidate`).
- CSS: `.next-word-dock*` / `.next-word-chip*` at the end of
  [40-candidates.css](../../../assets/css/40-candidates.css).

### 3. Theme → "Refined dark · slate + teal" ([00-tokens.css](../../../assets/css/00-tokens.css))
User rejected the Silk Veil purple+orange and picked a slate+teal direction from
a `/design` mockup. Applied by:
- Rewriting the tokens in `00-tokens.css` (bg `#14181d`, ink `#e8ebee`, muted
  `#a9b1ba`, accent `#6db6c9`, accent-strong `#8fcfdf`, accent-soft/active teal).
- **The theme was NOT fully tokenized** — ~41 hardcoded `rgba(233,138,78,…)`
  (orange) and ground hexes (`#1c1622`, `#14101b`, etc.) were scattered across
  the partials. All were mechanically replaced with teal/slate equivalents.
  Verified: `grep -rn "233, *138, *78\|#e98a4e\|#1c1622\|#14101b" assets/css/`
  returns nothing outside `00-tokens.css`.

## Where to improve (the actual ask)

The functional redesign is in; the **polish and craft** are open. Priorities:

1. **Finish tokenizing the theme.** New CSS should use `var(--accent)` etc., never
   hardcoded hex — so the next theme change is a one-file edit. Sweep the
   partials and replace any remaining literal colors with tokens.
2. **Editor surface & typography.** The editor is a plain `<textarea>` on a
   `--surface-strong` card. Refine: line-height for Khmer, caret/composition
   treatment, focus state, spacing rhythm. Match the `/design` mockup's calm
   slate feel (see the design canvas — ask the user for the link, or the working
   files under `khmerime-lab/prototypes/dioxus-editor-redesign/`).
3. **Docked next-word bar craft.** It works and is themed, but tune: chip sizing,
   the top-pick emphasis, the label, empty/appear transitions, and how it reads
   next to the candidate bar below it. Keep it **pointer-only** and never let it
   steal Enter/Tab/Space.
4. **Toolbar.** Mode pills, the font stepper, and the `Live Edit / Word / Manual /
   Rules / Saved` row. The responsive long/short label swap
   (`toolbar-label-long`/`-short` in [20-toolbar.css](../../../assets/css/20-toolbar.css))
   only works once CSS loads (it does now) — verify it at narrow widths.
5. **Hint row.** Now chip-based keycaps (`Space cycle · 1–5 choose · …`). Confirm
   it reads cleanly and matches the theme.

## Constraints / gotchas

- **CSS is embedded at compile time** (`include_str!`), so a CSS edit needs a
  rebuild (`dx serve` hot-reloads on save). Do NOT reintroduce
  `document::Stylesheet { href: … }` for `assets/` files — it will 404 under
  `dx serve` and silently break all styling.
- **Editor is a `<textarea>`** — no per-character inline styling. True inline
  "ghost text" was explored and rejected (would need contenteditable, which
  risks the Khmer IME composition core). Don't go there without a deliberate,
  TDD'd plan.
- Two candidate surfaces exist: the caret **`suggestion-popup`** (desktop
  transliteration) and the **`candidate-bar` / `candidate-track-mobile`**
  (mobile). The new `next-word-dock` is separate from both.
- `CandidateMode` (`Transliteration` | `NextWord`) drives which surface shows.
  Next-word prediction data comes from the engine's `next_word_suggestions`
  (KHPOS bigram stats, embedded — works without the `fetch-data` feature).

## Browser regression coverage

- Added Playwright regressions in `tests/test_web_ui.py` for both invariants:
  **Enter inserts a newline while next-word predictions are showing**, and the
  same predictions render only in `next-word-dock` (including mobile). The
  current local Python environment does not have Playwright installed, so these
  tests are committed as coverage but were not run in this session.

## Design source of truth

The earlier `/design` canvas remains historical reference for the docked-bar
interaction. ADR-0001 (webapp) and the focused-document grill are now the source of truth
for layout and theming. Working files:
`khmerime-lab/prototypes/dioxus-editor-redesign/` (Main, NextWordA/B, ThemeLight/
Slate/Ink `.dc.html`). The user picked **NextWord A (docked bar)** and **Theme
Slate (refined dark · slate + teal)**.
