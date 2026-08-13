# Khmer Flick Keyboard — MVP

A throwaway prototype of a Japanese-kana-style **flick keyboard** for Khmer.
15 keys in a 3×5 grid; each key holds up to 5 glyphs reached by flicking.

The prototype uses mobile keyboard conventions: a three-slot candidate strip,
the production mobile Quick Access sign tray, staggered sculpted keycaps,
familiar globe/number/space/return/delete controls, safe-area padding, and a
compact layout that adapts to short phone screens.

## Run it

Open `index.html` in a browser, or serve the folder:

```
cd experiments/khmer-flick-keyboard
python3 -m http.server 8000    # then visit http://localhost:8000
```

Works on a phone (touch) or desktop (mouse drag).

## How it works

- **Tap a key** → commits its **center** glyph.
- **Press and drag** toward up / left / right / down → a 5-way popup shows the
  choices; release on a direction to commit that glyph. Drag under ~22px = tap.
- Khmer-labelled space, return, and `⌫` backspace (deletes one code point).
- Globe and `123` are visual mode-switch placeholders in this layout prototype.
- **Word suggestions**: as you type Khmer, a bar above the keyboard shows words
  that start with the current word (prefix match). Tap one to complete it. The
  accepted suggestion creates an invisible boundary so the following joined
  Khmer word gets its own suggestions without inserting a space.
- **Quick Access signs**: the scrollable row mirrors the Android/iOS sign order.
  Combining marks use a display-only dotted circle and insert the raw mark.

## Suggestions

The suggestion bar prefix-matches the **current word** against `words.js` — 23k
words extracted from the RAC 2022 Khmer dictionary. Because Khmer words are
normally joined without spaces, selecting a suggestion records an invisible
boundary for the next prefix. Fully manual text is not auto-segmented in this
prototype. Matching is a plain `startsWith` scan, not a trie: instant at this
size, and a trie would be more code for no perceptible gain. Swap in a trie only
if the list grows into the hundreds of thousands.

Regenerate `words.js` from the dictionary if needed (extracts pure-Khmer `t_main`
entries, length 2–12):

```
# from repo root — see the python snippet in git history / ask, ~10 lines
```

There is **no ranking** yet (dictionary order); no frequency weighting, no roman
input, no engine integration. This bar is a layout/UX sketch, not the real IME.

## Design decisions (from the grill)

1. **Uniform 5-way keys.** Every key is center + up/left/right/down, **≤5 glyphs**.
   No key overflows — the wide vowel rows in the draft are just 5 keys side by side.
2. **Composed sras kept as-is.** `ុះ / ះ / េះ / ោះ` live together on the `ំ` key
   (row 3, key 4) — no need to type them in two steps, and no >5-glyph keys.
3. **Flick-commit gesture** (not tap-cycle) — the whole point of the design.
4. **Row 2 (independent vowels) is a best guess** — see below.

## Editing the layout

All glyphs live in **`keymap.js`** as plain data — edit any cell and reload.
Row 2 (independent vowels `ហ / ឫ / ឯ`) was filled from the draft as best I could
and has some blank directions; correct those glyphs there. Coeng now occupies
row 2 key 3, while the `ឥ` group occupies row 3 key 5.

```js
{ c: 'ក', u: 'គ', l: 'ខ', r: 'ឃ', d: 'ង' }   // center, up, left, right, down
```

## Not in the MVP (later)

- Grapheme-cluster backspace (base+sign deletes as one).
- Long-flick / second layer (not needed — nothing exceeds 5 glyphs).
- Real key-repeat, haptics, sound, theming, coeng (្) subscript stacking logic.
- Any integration with the actual khmerime engine — this is a layout sketch only.
