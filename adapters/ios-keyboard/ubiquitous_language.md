# Ubiquitous Language — iOS Keyboard Adapter

Terms used consistently across Swift UI code, Rust FFI layer, and conversations about this adapter.

---

## Keyboard State

The currently visible keyboard view. Exactly one state is active at all times.

| State | Description |
|---|---|
| **QWERTY** | Default roman-input view. 💡 in shift position, `123 space . ⏎` bottom row. |
| **123** | Number/symbol layer. Native iOS layout. All keys go through the session. |
| **#+=** | Secondary symbol layer, reached from 123. Full native iOS layout. |
| **Panel** | Candidate panel. Replaces QWERTY area. Shows segment chips + candidate row. |

Transitions: `QWERTY ↔ 123 ↔ #+=` and `QWERTY ↔ Panel` (via 💡).

---

## Strip

The two-row display pinned above the key rows, visible in all keyboard states.

- **Roman Row** — top line. Shows the segmented roman input, e.g. `nhom · ttov · salarien`. When there are no segments yet, shows the raw roman buffer.
- **Khmer Row** — bottom line. Shows the best Khmer candidate per segment, e.g. `ខ្ញុំ  ទៅ  សាលារៀន`. When there are no segments, shows the top-ranked candidate for the whole composition.

The strip is always visible. It does not hide when the keyboard switches to 123, #+= , or Panel.

---

## Composition

The in-progress roman input that has not yet been committed. Typing `nhom` starts a composition of four characters.

A composition ends when the user taps ⏎ (commits to Khmer) or clears the buffer via ⌫.

---

## Roman Buffer

Swift-side string mirroring the roman characters that have been inserted into the text field during the current composition. Used to:
1. Display the Roman Row when no segmentation is available yet.
2. Know how many `deleteBackward()` calls to make before inserting committed Khmer text.

The Roman Buffer is always in sync with what is physically in the text field — it is not the session's preedit.

---

## Session

The Rust-side `KhmerImeSession` (UniFFI-exported `KhmerIMESession`). Receives every key event and returns a `RenderState`. Owns romanization, segmentation, and candidate ranking.

Every key tap — letter, digit, symbol, space, backspace, return — is forwarded to the session. The session decides what to emit.

---

## Render State (`IosRenderState`)

The complete snapshot returned by the session after every key event. Contains:

- `preedit` — raw roman string the session is processing
- `segments` — list of `IosSegmentEntry` (roman input slice + Khmer output + focused flag)
- `candidates` — Khmer candidates for the currently focused segment
- `selectedIndex` — which candidate is highlighted
- `focusedSegmentIndex` — which segment owns candidate focus
- `commitText` — non-nil only immediately after ⏎; the concatenated Khmer string to insert

---

## Segment

A unit of a segmented composition. One segment maps one roman slice to one Khmer word.

Example: `nhomttovsalarien` → three segments: `nhom→ខ្ញុំ`, `ttov→ទៅ`, `salarien→សាលារៀន`.

A segment has:
- **Input** — the roman slice (e.g. `ttov`)
- **Output** — the best Khmer candidate for this slice (e.g. `ទៅ`)
- **Focused** — whether candidate navigation is currently on this segment

---

## Focused Segment

The segment currently receiving candidate navigation. Tapping a Segment Chip in the Panel moves focus to that segment. ← / → key events move focus left/right between segments.

---

## Segment Chip

A tappable button in the Panel representing one segment. Displays the segment's Khmer output. Tapping it moves focus to that segment and updates the Candidate Row.

---

## Candidate Row

The horizontally scrollable row in the Panel showing all Khmer candidates for the focused segment. Tapping a candidate commits that segment immediately.

---

## Panel (💡 Panel / Candidate Panel)

The full-replacement keyboard view activated by the 💡 button. Layout:

```
┌──────────────────────────────────────────────┐
│  strip (roman row / khmer row)               │
├──────────────────────────────────────────────┤
│  [ ខ្ញុំ ]  [ ទៅ ]  [ សាលារៀន ]   ← chips      │
├──────────────────────────────────────────────┤
│  ខ្ញុំ   ញុំ   ណុំ   ណ៉ំ  …          ← candidates │
├──────────────────────────────────────────────┤
│   123   │        space        │  .  │   ⏎   │
└──────────────────────────────────────────────┘
```

Tapping 💡 again (or a letter key) returns to QWERTY.

---

## Commit

The act of finalizing a composition into the text field:
1. Delete all roman characters in the Roman Buffer (`deleteBackward` × buffer length).
2. Insert the concatenated Khmer string (`insertText`).
3. Reset the Roman Buffer and clear the strip.

Khmer segments are concatenated **without spaces** (Khmer script convention).

Commit is triggered by ⏎. Tapping a candidate in the Panel commits only that segment (partial commit), not the entire composition.

---

## Passthrough

A key event that the session emits unchanged. Symbols (`-`, `/`, `(`, etc.) have no Khmer equivalent and are passed through as-is. Digits are not passthrough — the session converts `0–9` to Khmer digits `០–៩`.

---

## 💡 Button

The toggle button in the shift-key position (row 3, left). Switches between QWERTY and Panel states. Does not send any event to the session.

---

## ⏎ Button

The return/submit key. Label: `⏎` glyph. Triggers a full Commit of the current composition.

---

## 123 Button

Switches keyboard state from QWERTY → 123 (or Panel → 123). Label: `123`. Does not send any event to the session.

---

## ABC Button

Returns keyboard state to QWERTY from 123 or #+=. Label: `ABC`. Does not send any event to the session.
