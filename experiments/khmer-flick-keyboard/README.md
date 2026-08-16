# Khmer Directional Lean Keyboard — MVP

A throwaway prototype of a compact **Directional Lean keyboard** for Khmer.
It has 15 keys in a 3×5 grid; each key holds a center member and up to four
direction members.

The prototype uses mobile keyboard conventions: a three-slot candidate strip,
the production mobile Quick Access sign tray, sculpted keycaps,
number/space/return/delete controls, safe-area padding, and a compact layout that
adapts to short phone screens.

The header includes independent, persistent display switches. **Night**
changes the full interface palette; **Heat** controls whether prediction rings
are visible without changing the underlying n-gram calculations; **Centre only**
hides direction hints so each key displays only its large centre character and
compacts the key rows to use less screen space; **`យត`** switches to Layout B,
where `យ` replaces `រ` and `ត` replaces `ន` as their family centres.

Khmer text uses **Kantumruy Pro** from Google Fonts so Android browsers render
the same stronger, screen-readable glyphs instead of choosing a device fallback.
If the webfont cannot load, the app falls back to Noto Sans Khmer and then the
device sans-serif font.

## Transition-optimized layout

The center keys are arranged by expected thumb movement through real corpus
transitions, with the proposed vowel row fixed:

```text
ើ   េ   ុ   ា   ះ
ដ   ក   រ   ន   ច
ឯ   ស   ្   ប   ោះ
```

The full inputs, assumptions, transition tables, coeng traffic, and objective
score are recorded in [`LAYOUT_ANALYSIS.md`](LAYOUT_ANALYSIS.md).

Recompute the documented score—or test a two-family swap—without editing the
app:

```sh
node analyze-layout.mjs
node analyze-layout.mjs --swap រ ្
```

Within each family, the most frequent member is the center tap. Direction members
are then assigned by frequency and gravity toward the geometric center key `រ`.
The same map works unchanged for either hand. The exact percentages and complete
direction table are recorded in the analysis.

## N-gram heatmap (next-key prediction)

At rest, the keys softly show statistically useful places to begin. After each
character, they switch to what is likely to come next, using a Khmer trigram
back-off model measured on a 31.4M-character corpus.

- **Start guidance:** filtered unigram frequency includes only Khmer consonants
  and independent vowels. It excludes coeng and dependent vowels, which are
  globally frequent but cannot start a word. At the current 2% floor this softly
  highlights the `ន`, `រ`, `ក`, `ប`, and `ស` families.
- **Context:** once composition starts, P(next | last two glyphs) backs off from
  trigram to bigram (last glyph).
- **Granularity:** on the resting board each key is scored by its *most likely*
  glyph (`max`, not sum — so a key with one likely direction member still lights).
- **Visual:** a glowing border ring on the **top ~6** likely keys, ring brightness
  proportional to probability; start guidance is softer than contextual heat.
  Across those keys, only the **top 8 individual characters** receive a tight
  compact translucent background glow. Those eight are re-ranked onto the full heat scale: rank
  1 is red, the middle ranks cross yellow/green/cyan, and rank 8 is blue. Opacity
  also fades by rank, keeping the exact character guidance informative without
  marking every member. Key fills remain unchanged, and heat ranking does not
  carry into the Lean Preview.
- **Narrow screens:** at 380 CSS px and below, direction members become smaller
  and quieter while key height increases slightly. At 340 px and below, idle
  direction members are hidden and remain discoverable in the Lean Preview.
- **Lean Preview:** touching a key expands its complete family above the finger;
  the selected member follows the lean direction and receives the solid accent.
- **Data:** `ngram.js` — pruned `TRIGRAM` / `BIGRAM` / `UNIGRAM` tables
  (`{context: {next: prob}}`), about 195 KB. Regenerate from the corpus as needed.

Why trigram and not just frequency: after COENG ្, for example, the next character
is 100% a consonant, and `រ` alone accounts for ~30% — context sharpens the hint far
beyond static letter frequency.

## Run it

Open `index.html` in a browser, or serve the folder:

```
cd experiments/khmer-flick-keyboard
python3 -m http.server 8000    # then visit http://localhost:8000
```

Works on a phone (touch) or desktop (mouse movement while pressed).

## How it works

- **Press a key** → a one-character Lean Preview immediately shows its center.
- **Lean while holding** toward up / left / right / down → the preview follows
  the finger and shows the provisional target.
- The first 10px around contact is the **Neutral Zone**. Returning there restores
  the center. A missing direction also keeps the center selected.
- Movement never cancels the gesture. The final release vector chooses and
  commits the character, regardless of whether release occurs inside the key.
- Khmer-labelled space, return, and `⌫` backspace (deletes one code point).
- `123` is a visual mode-switch placeholder in this layout prototype.
- **Copyable output**: typed text supports normal selection; the top-right
  `ចម្លង` button copies committed text and the active preedit together.
- **Word suggestions**: as you type Khmer, a bar above the keyboard shows words
  that start with the current word (prefix match). Tap one to complete it. The
  accepted suggestion creates an invisible boundary so the following joined
  Khmer word gets its own suggestions without inserting a space.
- **Quick Access signs**: `់`, `៉`, and `។` moved to direction members on the coeng key
  without duplication. The scrollable row starts `ឲ្យ ៏ ័ ៈ ៍ ៌ ៊ ៗ …`.
  Combining marks insert the raw mark.

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
2. **Composed sras kept as-is.** `ុះ / េះ / ោះ` remain one-gesture entries—no
   need to type them in two steps, and no >5-glyph keys.
3. **Directional Lean gesture** (not tap-cycle): subtle movement selects a
   direction while a single-character preview confirms the pending result.
4. **Transition placement, inward gravity within keys.** Corpus transitions
   determine key position. Frequency determines the center tap and direction rank;
   higher-ranked members point toward `រ` before lower-ranked members point out.
5. **Fast marks join coeng.** `់`, `៉`, and `។` are direct direction members of `្` and
   are removed from Quick Access, so there are no duplicate inputs.

## Editing the layout

All glyphs live in **`keymap.js`** as plain data—edit any cell and reload. When
moving whole keys, update `LAYOUT_ANALYSIS.md` with the new statistical rationale
instead of describing the result as frequency-optimized.

```js
{ c: 'ក', u: 'ង', l: 'ឃ', r: 'ខ', d: 'គ' }   // center, up, left, right, down
```

## Not in the MVP (later)

- Grapheme-cluster backspace (base+sign deletes as one).
- Long-lean / second layer (not needed — nothing exceeds 5 glyphs).
- Real key-repeat, haptics, sound, theming, coeng (្) subscript stacking logic.
- Any integration with the actual khmerime engine — this is a layout sketch only.
