# Transition layout analysis

The selected layout combines corpus transition statistics with Inward Gravity
for direction members. It remains unchanged when used by either hand.

```text
ើ   េ   ុ   ា   ះ
ដ   ក   រ   ន   ច
ឯ   ស   ្   ប   ោះ
៕   ឲ្យ   ៏   ៈ   ឮ
```

The five row-1 families are fixed by the design discussion and live thumb
testing. The seven consonant families and coeng occupy the statistically selected
positions. The rare independent-vowel and composed-vowel families fill the two
lower corners; their most frequent members, `ឯ` and `ោះ`, are now their centers.

## Data and layout objective

- Source: `ngram.js` and `char-frequency.txt`, measured over 31,426,551 Khmer
  code points from the kmwiki corpus.
- Transition mass: `P(a) × P(b | a)` from `UNIGRAM` and `BIGRAM`.
- Fixed row: `ើ េ ុ ា ះ`; live testing overrode the original order of the
  `ើ` and `េ` families.
- Search space: seven consonant families plus the enlarged coeng family across
  rows 2–3, with the two rare corner families occupying the remaining slots.
- Travel cost: expected Euclidean key-to-key distance, with horizontal distance
  weighted `1.12×` and row pitch `1.05`.
- Reach cost: a neutral lower-center reach penalty weighted `0.08×`.
- Search: 16 deterministic simulated-annealing restarts, 350,000 swaps each.

Run the checked-in calculation with:

```sh
node analyze-layout.mjs
node analyze-layout.mjs --swap រ ្
node analyze-layout.mjs --centers yt --lean-scale 0.25
```

The current center layout has this objective under the explicitly assumed
parameters above:

```text
expected transition travel            1.78796
reach                                  1.53457
weighted reach contribution  0.08 ×   1.53457 = 0.12277
                                                   -------
modeled effort                                    1.91073
modeled transition mass                           0.91829
```

Reach is now reproducibly defined using the same set of modeled characters in
both numerator and denominator:

```text
R = Σ P(a) d(key(a), thumbHome) / Σ P(a)
thumbHome = (2, 2), the lower-center coeng-key position
```

The previously recorded `R = 2.10917` mixed different character sets between
its numerator and denominator and cannot be reproduced consistently. It has
been replaced rather than preserved as a valid result.

`1.91073` is an arbitrary normalized effort score for comparing layouts under
this model. It is not a typing time, distance in centimeters, or percentage. A
lower score means the modeled transitions require less thumb travel and/or
reach.

The n-gram data has no explicit word-boundary token, so word-initial and
word-final placement is represented indirectly through observed adjacent
characters.

## Centre-character A/B comparison

The app provides a manual Layout B switch that directly swaps `រ ↔ យ` and
`ន ↔ ត` within their existing families. Key positions and family membership do
not change. The key-to-key objective therefore gives both layouts the same
`E = 1.91072919`.

To compare centre selection provisionally, the analysis script also reports:

```text
S = Σ P(g) q(direction(g)) / Σ P(g)
q(center) = 0
q(up/down) = 1.05
q(left/right) = 1.12
Eₖ = E + kS
```

`k` is an assumed scale for within-key leaning, not an experimentally validated
ergonomic constant. With the illustrative `k = 0.25`:

| Variant | Centres | Directional rate | `S` | `Eₖ` |
|---|---|---:|---:|---:|
| A | `រ`, `ន` | 47.16% | 0.50655 | 2.03737 |
| B | `យ`, `ត` | 52.83% | 0.56815 | 2.05277 |

Under every positive `k`, Layout A has lower modeled mechanical effort. Layout
B remains useful for testing whether recognition or memorability offsets its
higher directional-selection rate.

## Member ranking and inward gravity

Family membership stays fixed. Within each family:

1. The highest-frequency member is the center tap.
2. The geometric target is the `រ` key at row 2, column 3.
3. Higher-ranked direction members take directions that point toward that target.
   Inward vertical/horizontal directions are filled before outward directions.
4. Incomplete families populate only center-facing directions where possible.
5. The `ើ` family keeps the experience-tested exception `ៀ` on down.

For composed entries, frequency is estimated from adjacent-code-point
probability: `P(first) × P(second | first)`. This gives approximately `ោះ`
`0.579%`, `េះ` `0.412%`, and `ុះ` `0.130%`. Single-code-point percentages below
come directly from `char-frequency.txt`.

| Family | Members ranked by corpus frequency |
|---|---|
| `ើ` | `ើ` 1.474%, `ៀ` 0.256% |
| `េ` | `េ` 1.571%, `ោ` 1.443%, `ែ` 1.038%, `ៅ` 0.688%, `ៃ` 0.305% |
| `ុ` | `ុ` 2.067%, `ូ` 1.122%, `ួ` 0.722%, `ឿ` 0.075% |
| `ា` | `ា` 8.289%, `ិ` 2.310%, `ី` 1.257%, `ឹ` 0.455%, `ឺ` 0.166% |
| `ះ` | `ះ` 1.694%, `ំ` 1.608% |
| `ដ` | `ដ` 1.918%, `ណ` 0.764%, `ឋ` 0.121%, `ឌ` 0.057%, `ឍ` 0.006% |
| `ក` | `ក` 4.394%, `ង` 3.335%, `គ` 1.458%, `ខ` 0.908%, `ឃ` 0.137% |
| `រ` | `រ` 5.275%, `ល` 2.619%, `យ` 2.493%, `វ` 1.463% |
| `ន` | `ន` 6.022%, `ត` 3.328%, `ទ` 2.309%, `ធ` 0.656%, `ថ` 0.630% |
| `ច` | `ច` 1.888%, `ជ` 1.468%, `ញ` 0.750%, `ឆ` 0.168%, `ឈ` 0.129% |
| `ឯ` | `ឯ` 0.124%, `ឧ` 0.087%, `ឬ` 0.069%, `ឥ` 0.054%, `ឱ` 0.050% |
| `ស` | `ស` 3.516%, `អ` 1.281%, `ហ` 0.792%, `ឡ` 0.376% |
| `្` | `្` 9.113%, `់` 1.908%, `។` 0.707%, `៉` 0.296% |
| `ប` | `ប` 3.728%, `ម` 3.665%, `ព` 2.082%, `ភ` 0.596%, `ផ` 0.350% |
| `ោះ` | `ោះ` ~0.579%, `េះ` ~0.412%, `ុះ` ~0.130% |

The resulting directions are the implementation contract:

| Center | Up | Left | Right | Down |
|---|---|---|---|---|
| `ើ` | — | — | — | `ៀ` |
| `េ` | `ៅ` | `ៃ` | `ែ` | `ោ` |
| `ុ` | — | `ឿ` | `ួ` | `ូ` |
| `ា` | `ឹ` | `ី` | `ឺ` | `ិ` |
| `ះ` | — | `ំ` | — | — |
| `ដ` | `ណ` | `ឍ` | `ឌ` | `ឋ` |
| `ក` | `ង` | `ឃ` | `ខ` | `គ` |
| `រ` | `ល` | `យ` | `វ` | — |
| `ន` | `ត` | `ធ` | `ថ` | `ទ` |
| `ច` | `ជ` | `ឆ` | `ឈ` | `ញ` |
| `ឯ` | `ឧ` | `ឱ` | `ឬ` | `ឥ` |
| `ស` | `អ` | `ឡ` | `ហ` | — |
| `្` | `់` | `៉` | `។` | — |
| `ប` | `ម` | `ព` | `ផ` | `ភ` |
| `ោះ` | `េះ` | `ុះ` | — | — |

For example, the `ប` family points its two highest-frequency direction members toward the
center: `ម` upward and `ព` left. The rarer `ផ` and `ភ` occupy the outward right
and down directions. Likewise, `ិ` is more frequent than `ី`, but on the top-row
`ា` key the down direction points more strongly toward the center, so `ិ` goes
down and `ី` goes left.

## Quick Access migration

`់` (bantoc, 1.908%), `។` (khan, 0.707%), and `៉` (muusikatoan, 0.296%) moved
from Quick Access into the coeng family without duplication. This puts frequent
marks on the main grid while keeping coeng at the lower center. Before the
remaining items moved into the Auxiliary Family Row, the tray began:

```text
ឲ្យ  ៏  ័  ៈ  ៍  ៌  ៊  ៗ  …
```

### Auxiliary Family Row

The remaining 25 browser-experiment Quick Access items will replace the
scrolling tray with five fixed Key Families in an **Auxiliary Family Row** below
the 3×5 core. Membership follows linguistic/function relationship before
frequency or geometry is considered:

| Family | Fixed members |
|---|---|
| Independent vowels A | `ឮ`, `ឫ`, `ឪ`, `ឭ`, `ឰ` |
| Independent vowels B | `ឲ្យ`, `ឦ`, `ឳ`, `ឩ`, `ឨ` |
| Orthographic modifiers | `៏`, `័`, `៍`, `៌`, `៊` |
| Reading/phrase signs | `ៈ`, `ៗ`, `៎`, `៖`, `៑` |
| Rare terminal symbols | `៕`, `៛`, `៚`, `៙`, `៘` |

Family membership is fixed before position optimization; it does not change to
improve the effort score.

`ឲ្យ` remains an atomic multi-character shortcut rather than being replaced by
raw `ឲ`. It is the centre of Independent Vowels B: the character-level corpus
uses `P(ឲ)` as an explicit frequency proxy, which is much larger than the
frequencies of the other four members. This proxy is provisional until
phrase-level shortcut usage is measured.

Exhaustive search over all `5! = 120` horizontal family orders fixes the bottom
row, left to right, as:

```text
Rare terminal symbols | Independent vowels B | Orthographic modifiers | Reading/phrase signs | Independent vowels A
```

Using full `char-frequency.txt` unigram mass, the pruned bigram table, auxiliary
row `y = 3`, and the same assumed model constants gives `E = 1.92765975`. With
the illustrative within-key scale `k = 0.25`, `E + kS = 2.05573273`. These are
the best scores among the 120 fixed-membership orders, not experimentally
validated ergonomic measurements.

Accepted Auxiliary Family Row directions:

| Position | Family | Center | Up | Left | Right | Down |
|---:|---|---|---|---|---|---|
| 0 | Rare terminal symbols | `៕` | `៚` | `៘` | `៛` | `៙` |
| 1 | Independent vowels B | `ឲ្យ` | `ឦ` | `ឩ` | `ឳ` | `ឨ` |
| 2 | Orthographic modifiers | `៏` | `័` | `៍` | `៌` | `៊` |
| 3 | Reading/phrase signs | `ៈ` | `ៗ` | `៎` | `៖` | `៑` |
| 4 | Independent vowels A | `ឮ` | `ឪ` | `ឫ` | `ឰ` | `ឭ` |

The core and auxiliary families share one predictive heatmap. All 20 families
compete for the same top-six family rings, and all members compete for the same
global top-eight radial glows. `ឲ្យ` uses raw `ឲ` as its prediction proxy; no
auxiliary family receives permanent emphasis merely because of its row.

The Auxiliary Family Row remains visible before, during, and after composition.
Several of its modifiers are needed after a base character, so hiding the row
when preedit begins would make valid direct-entry sequences unreachable.

Auxiliary keys use the same height and Directional Lean geometry as core keys.
The row sits immediately below the three core rows and above the action row;
shrinking it would save height at the cost of smaller Khmer labels and a less
consistent motor target.

One Backspace removes one Entry Unit while composing. Consequently, a single
selection of `ឲ្យ`, `ោះ`, `េះ`, or `ុះ` is also undone by a single Backspace;
committed-text deletion remains separate from this experimental preedit rule.

## Vowel-family transition profile

Each row is normalized over transitions from that consonant family into the five
fixed row-1 families.

| Family | `ើ` | `េ` | `ុ` | `ា` | `ះ` | Horizontal barycenter |
|---|---:|---:|---:|---:|---:|---:|
| `ក` | 4.9% | 15.1% | 18.6% | 55.2% | 6.2% | 2.43 |
| `ន` | 2.0% | 30.9% | 16.3% | 48.6% | 2.2% | 2.18 |
| `ប` | 2.9% | 12.5% | 19.3% | 64.3% | 1.1% | 2.48 |
| `រ` | 13.0% | 15.2% | 17.1% | 42.4% | 12.3% | 2.26 |
| `ស` | 17.2% | 19.5% | 11.5% | 43.4% | 8.4% | 2.06 |
| `ច` | 1.4% | 8.9% | 17.2% | 64.8% | 7.7% | 2.68 |
| `ដ` | 8.7% | 46.6% | 11.0% | 30.5% | 3.2% | 1.73 |

## Absolute traffic into row 1

| Family | Joint transition mass |
|---|---:|
| `ន` | 0.08298 |
| `រ` | 0.06554 |
| `ប` | 0.04804 |
| `ស` | 0.02679 |
| `ក` | 0.02652 |
| `ច` | 0.02417 |
| `ដ` | 0.01829 |

## Bare-coeng bridge traffic

This table isolates traffic to and from the bare `្` code point; the objective
above scores the whole enlarged coeng family.

| Family | Family → `្` | `្` → family |
|---|---:|---:|
| `ក` | 0.03488 | 0.00816 |
| `ន` | 0.02465 | 0.02204 |
| `ប` | 0.02727 | 0.00773 |
| `រ` | 0.00227 | 0.04042 |
| `ស` | 0.01068 | 0.00380 |
| `ច` | 0.00718 | 0.00400 |
| `ដ` | 0.00275 | 0.00297 |
