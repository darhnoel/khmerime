# Transition layout analysis

The selected layout combines corpus transition statistics with an inward-gravity
rule for the member flicks. It remains unchanged when used by either hand.

```text
ើ   េ   ុ   ា   ះ
ដ   ក   រ   ន   ច
ឯ   ស   ្   ប   ោះ
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

## Member ranking and inward gravity

Family membership stays fixed. Within each family:

1. The highest-frequency member is the center tap.
2. The geometric target is the `រ` key at row 2, column 3.
3. Higher-ranked flick members take directions that point toward that target.
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

For example, the `ប` family points its two highest-frequency flicks toward the
center: `ម` upward and `ព` left. The rarer `ផ` and `ភ` occupy the outward right
and down directions. Likewise, `ិ` is more frequent than `ី`, but on the top-row
`ា` key the down direction points more strongly toward the center, so `ិ` goes
down and `ី` goes left.

## Quick Access migration

`់` (bantoc, 1.908%), `។` (khan, 0.707%), and `៉` (muusikatoan, 0.296%) moved
from Quick Access into the coeng family without duplication. This puts frequent
marks on the main grid while keeping coeng at the lower center. Quick Access now
begins:

```text
ឲ្យ  ៏  ័  ៈ  ៍  ៌  ៊  ៗ  …
```

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
