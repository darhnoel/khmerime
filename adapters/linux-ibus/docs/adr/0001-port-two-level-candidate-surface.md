# Porting the two-level Candidate Surface + roman-header layout to Linux IBus

Status: proposed (handoff — not yet implemented)

Windows TSF (`windows-tsf` ADR-0002) and then macOS IMK (`macos-imk` ADR-0004) reworked how a
**Segmented Session** presents candidates. Linux IBus has not yet adopted it. This document records
what those two adapters now do, what IBus already has for free, and the concrete work left so IBus
reaches parity. It is the porting brief for that work.

**Scope note:** the AI span-proposal model (`SpanProposalMode::Model` / Tonle) is live only on the
Apple builds today. Bringing it to Windows and Linux is a **separate, later iteration** and is out of
scope here. Everything below is deterministic-engine presentation, which does not depend on the model.

## The Candidate Surface concept (platform-neutral, in CONTEXT.md)

A Segmented Session has **two levels** of candidate:

- **Phrase level** — the whole-phrase candidates (`candidates` when `segmented_active` and not editing).
- **Segment level** — alternatives for the *one focused segment*, shown after Tab enters Segment Edit
  Mode (`segment_edit_active`).

Showing both at once made a correct *word* alternative (e.g. ខ្ចុំ for the focused ខ្ញុំ) look like a
wrong *phrase* alternative for the complete phrase (ខ្ញុំទៅ). The fix is to present **one level at a
time**, mode-switched. This is a per-adapter *presentation* policy; ranking stays in the shared engine.

```text
Flat mode              Phrase mode (default, segmented)   Segment mode (after Tab)
single-segment         rows = whole Phrase Candidates      rows = focused segment's words
no header              two-row header (Khmer / roman)      same header, focused word boxed
roman kept per row     roman ONLY in header                roman ONLY in header
```

## What the shared session already gives IBus (no work needed)

These were fixed in `crates/session` and reach every adapter, IBus included. IBus's
`ibus_bridge_protocol` tests already pass with the new behavior:

- **Space in Segment Edit Mode cycles the focused word and never commits** (`handle_space` →
  `cycle_candidates`). Enter is the explicit commit.
- **Left/Right move segment focus only *after* Tab**, and stay in Segment Edit Mode. Before Tab they
  are consumed but inert — they do not move focus (so the first Tab always lands on segment 0). See
  `left_right_before_tab_are_consumed_but_do_not_move_focus` in `segment_edit_mode.rs`.

The JSON snapshot IBus already receives over stdio carries everything the UI needs — no bridge/Rust
change is required for this port. Relevant fields (`fallback_empty_snapshot_json` in
`adapters/linux-ibus/src/lib.rs` lists the full shape):

- `segmented_active`, `segment_edit_active`, `focused_segment_index`, `segment_edit_index`
- `segment_preview`: list of `{output (Khmer), input (roman), focused (bool)}` — one per segment
- `candidates` + `candidate_display`: the rows, each display entry has `output`, `recommended`,
  `roman_hints`, `is_raw_fallback`

So this is a **Python-renderer-only** change on the IBus side.

## What IBus does today (the gap)

IBus renders differently from the Apple/Windows model, and this is what changes:

1. **Segments are one-line auxiliary text with roman inlined in parens.**
   `ibus_segmented_preview_renderer.build_segment_preview` produces `output(input)` per chunk joined by
   ` | `, focused chunk wrapped in `⟦ ⟧`. The roman is glued onto each Khmer chunk on a single line.

2. **Candidate rows repeat the roman on every row.**
   `ibus_candidate_renderer.candidate_rows` renders `output (hint1 / hint2)` — the exact per-row roman
   repetition macOS just removed. Every row shows the same roman, wasting the lookup-table width.

3. **No mode switch.** IBus does not distinguish Phrase vs. Segment vs. Flat; it shows the segment
   preview (aux text) plus the candidate list together, which is the "both levels at once" problem.

IBus has a real constraint the desktop popups don't: it renders through **IBus's own auxiliary text +
lookup table**, not a custom-drawn panel. The two-row aligned header macOS draws with chips is not
directly expressible — IBus aux text is a single styled string. So the port adapts the *intent*, not
the pixels (see "IBus-specific decisions" below).

## The target behavior for IBus

Derive a **surface mode** from the snapshot (same rule as macOS's `CandidateSurface::from_snapshot`):

| Condition                                          | Mode    |
| -------------------------------------------------- | ------- |
| `segmented_active` && !`segment_edit_active`       | Phrase  |
| `segment_edit_active`                              | Segment |
| otherwise (single-segment composition)             | Flat    |

Then:

- **Phrase & Segment mode:** candidate rows are **Khmer-only** — drop the per-row `(roman)` hint. The
  roman lives in the segment preview (aux text) instead. Keep the `≈` derived marker and `✓`
  recommended marker as today.
- **Flat mode:** keep the per-row roman hint (`output (hint)`) — there is no header carrying it, so it
  is the only place the roman shows. This mirrors macOS's Flat-mode exception exactly.
- **Segment preview (aux text):** keep it as the roman-carrying header. Consider dropping the inline
  `output(input)` gluing in favor of showing the roman for the *focused* segment more prominently — but
  the aux text is already the natural home for roman, so the minimum change is: **just stop repeating
  roman on the candidate rows and let the aux text keep it.**

## IBus-specific decisions (the "practical, not pixel-identical" calls)

- **No two-row aligned chip header.** IBus aux text is a single line. Keep the existing
  ` | `-separated `output(input)` preview as the roman-carrying line. The macOS win (roman off every
  candidate row, kept in one header place) is achieved by fixing #2 above; the *shape* of the header
  stays IBus-native. Do not try to fake column alignment in aux text.
- **No dynamic panel width / horizontal scroll.** IBus owns the lookup-table geometry; the macOS width
  clamp and truncation are panel concerns that do not exist here. Skip them.
- **Overflow:** IBus's lookup table already handles long rows its own way; nothing to add.

## Suggested TDD slices (vertical, one test → one change)

The IBus renderers are pure Python with existing unit tests — ideal for red-green. Suggested order:

1. **Surface mode from snapshot.** New pure helper `surface_mode(snapshot) -> "flat"|"phrase"|"segment"`
   in a small module (or extend `ibus_render_plan`). Test the three conditions above.
2. **Khmer-only rows in Phrase/Segment mode.** Give `candidate_rows` the mode; assert it drops the
   `(roman)` suffix when mode is phrase/segment, and keeps it in flat. Reuse the macOS test cases:
   `output="អ្នកបន្ថែមទៀត"` → row is exactly `អ្នកបន្ថែមទៀត` in phrase mode; `≈`/`✓` markers survive.
   (No `✦` model marker on IBus yet — that arrives with the AI iteration.)
3. **Wire mode through the engine.** `khmerime_ibus_engine` computes the mode from the snapshot and
   passes it to `candidate_rows`. Cover with an `ibus_bridge_protocol`-style end-to-end assertion if
   practical, else a renderer-level test.

Keep the existing markers (`✓`, `≈`) and the raw-fallback escape hatch untouched.

## Cross-adapter parity table (for reviewers)

| Behavior                                  | TSF | macOS | IBus (target) |
| ----------------------------------------- | --- | ----- | ------------- |
| Two-level surface (mode switch)           | ✅  | ✅    | ⬜ this port  |
| Roman off candidate rows in Phrase/Segment| ✅  | ✅    | ⬜ this port  |
| Roman kept on rows in Flat mode           | ✅  | ✅    | ⬜ this port  |
| Space cycles word (never commits)         | ✅  | ✅    | ✅ (session)  |
| Left/Right inert before Tab               | ✅  | ✅    | ✅ (session)  |
| Two-row aligned chip header               | —   | ✅    | ➖ IBus aux text is single-line |
| Dynamic panel width / truncation          | —   | ✅    | ➖ IBus owns geometry |
| AI span proposals (Tonle)                 | ⬜ next iter | ✅ | ⬜ next iter |
