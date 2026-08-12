# Two-level Candidate Surface: phrase candidates before segment candidates

Status: accepted

The macOS candidate panel rendered `segment_preview` as chips and `candidates` as the
rows beneath them, both at once. During a **Segmented Session** those rows are alternatives
for only the *focused segment*, while the chips show the *whole phrase*. Putting both levels
in one unlabelled panel made a correct word alternative (e.g. ខ្ចុំ for the focused ខ្ញុំ)
look like a wrong alternative for the complete phrase (ខ្ញុំទៅ). Windows TSF hit the same
problem and fixed it with a two-level Candidate Surface (windows-tsf ADR-0002); macOS adopts
the same model. This is the platform-neutral **Candidate Surface** concept in CONTEXT.md.

## Decision

macOS presents one candidate level at a time, mode-switched:

```text
Phrase mode (default while segmented)   Segment mode (after Tab: Segment Edit)
-------------------------------------   -------------------------------------
rows = whole Phrase Candidates          rows = focused-segment Candidate List
↑/↓, Space cycle phrases                ↑/↓, Space cycle words (Space NEVER commits)
1..9 select a phrase                    1..9 select a word
segmentation shown as a dim,            ←/→ move to the adjacent segment AND
non-selectable context header           stay in Segment Edit on it
Enter commits the whole phrase          Enter commits the whole phrase; Tab exits to Phrase
```

Two shared-session behaviors were corrected for the keyboard-driven desktop adapters (macOS +
Windows) as part of this: in Segment Edit Mode, **Space cycles the focused word and never commits**
(it used to commit the whole composition — the bug), and **Left/Right move focus to the adjacent
segment and stay in Segment Edit** (they used to auto-exit). Mobile (iOS/Android) is unaffected:
its Space is an adapter-level commit (`spaceTapped`/`sendSpace`) that never routes through the
session's key handling.

Left/Right **only move segment focus once Segment Edit Mode is active** (after Tab). While segmented
but not yet editing, they are still *consumed* — they never leak to the document and disturb the
marked composition — but they are **inert**: they do not move `focused`. This matters because Tab
enters edit on `session.focused`; letting a pre-Tab arrow advance `focused` made the first Tab land
on the *second* segment instead of the first. Consuming-but-inert matches the Windows TSF rule,
which consumes Left/Right whenever a segmentation is active (`segmented_active`) but only navigates
segments inside edit mode.

A flat, single-segment **Composition** keeps the ordinary **Candidate List** (Flat mode) —
no second level, no context header.

## Header layout: roman on its own row, not repeated per candidate

The context header is **two aligned rows** — Khmer chunks on top, their roman chunks (`segment.input`)
directly beneath, one column per segment. In Phrase and Segment mode the roman lives *only* in this
header, so the candidate rows below are **Khmer-only** — the roman used to be appended to every row
(`អ្នកបន្ថែមទៀត  neak bonthaem tiet` repeated on each), which wasted space and was identical across
rows since every candidate reads the same roman. The roman header updates with the previewed phrase /
focused segment (it is the same `segment_preview` list that produces the Khmer row). Flat mode has no
header, so its candidate rows **keep** the per-row roman hint — it is the only place the roman shows.

Overflow: a long phrase's header chunks **truncate with an ellipsis** (like the candidate rows already
do); the panel is a keyboard-driven floating popup, so trackpad horizontal scroll is not used. If
long-phrase editing needs it later, the follow-up is keyboard-driven auto-scroll that keeps the
Tab/arrow-focused segment visible — never a trackpad scroll.

Phrase-level key override. The shared `khmerime_session`, when segmented but not in Segment
Edit Mode, cycles the *focused segment's words* on Space/arrows — not phrases. To make the
phrase level actually select phrases (there is no tap on a desktop IMK panel, unlike the
mobile Phrase Wheel), macOS ports TSF's `command_for_key`: at Phrase mode, Space/↑/↓ cycle
whole Phrase Candidates (`SessionCommand::SelectPhrase`) and 1..9 pick one. Tab, Left/Right,
Enter, and the Segment/Flat levels keep the shared session's existing behavior unchanged —
`command_for_key` returns `None` there and the key delegates to the session.

## Deep module boundary

A `CandidateSurface` projection lives on the **Rust side** of the adapter
(`adapters/macos-imk/src/lib.rs`), mirroring the TSF `render::candidate_surface` shape:
`from_snapshot` reads the existing `SessionSnapshot` and yields `mode`
(Flat / Phrase / Segment), `rows`, `selected_index`, and `context`. The `MacosRenderState`
carries that projection over UniFFI; the Swift `CandidatePanel` is a dumb painter that
shows the given rows and paints `context` as the header. The panel never inspects session
modes itself, so render and interaction cannot drift.

This is deliberately adapter presentation policy, not new ranking. `khmerime_session` still
owns Phrase Candidates, Segment Edit Mode, selection, and commits; macOS only chooses which
existing level to expose — matching the per-adapter pattern already used by iOS/Android
(Phrase Wheel) and Windows (Candidate Surface). Linux/IBus remains flat for now.

## Consequences

- The word-vs-phrase confusion is gone: rows always mean one thing, named by the mode.
- The projection + its key-command policy are unit-testable in plain Rust (no Xcode / paid
  project build friction), like TSF's `candidate_surface` tests.
- The UniFFI `MacosRenderState` gains a surface-mode + context split (additive).
- iOS/Android are untouched (they parse JSON snapshots and already have the Phrase Wheel);
  a future shared projection could serve the three desktop adapters, but that would reverse
  the current adapter-local stance and is out of scope here.
