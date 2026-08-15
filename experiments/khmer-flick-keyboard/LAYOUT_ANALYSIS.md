# Transition layout analysis

The selected center-key layout keeps the proposed first row fixed and minimizes
expected one-thumb movement through the remaining consonant families:

```text
ើ   េ   ុ   ា   ះ
ដ   ក   រ   ន   ច
ឱ   ស   ្   ប   ុះ
```

`ឱ` and `ុះ` occupy the two statistically unassigned lower corners so the
existing 15-key glyph inventory remains intact.

## Data and assumptions

- Source: the existing `ngram.js` model measured over 31,426,551 Khmer code
  points from the kmwiki corpus.
- Transition mass: `P(a) × P(b | a)` from `UNIGRAM` and `BIGRAM`.
- Fixed row: `ើ េ ុ ា ះ`; live right-thumb testing overrode the original order.
- Search space: seven consonant families plus coeng across rows 2–3, with two
  unused statistical slots.
- Cost: expected Euclidean key-to-key distance, horizontal distance weighted
  `1.12×`, row pitch `1.05`, plus a small (`0.08×`) neutral lower-center reach
  penalty.
- Search: 16 deterministic simulated-annealing restarts, 350,000 swaps each.
- Winning objective after the swap: `1.98785` arbitrary normalized effort units
  (`1.81572` expected transition travel + `0.17213` reach contribution), a 3.34%
  improvement over the previous `2.05642` arrangement.

The score compares layouts under this model; it is not a measured typing time.
The n-gram data has no explicit word-boundary token, so word-initial/final
placement is represented indirectly through observed adjacent characters.

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

## Hand-neutral member directions

The app uses one unchanged layout for either hand. The frequent non-center
members use vertical flicks, whose comfort does not reverse with handedness:

```text
ើ: down ៀ
េ: up ោ, down ែ, left ៃ, right ៅ
```

The `ុ`, `ា`, and `ះ` member directions remain unchanged so this experiment
isolates the two swapped groups.

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

## Coeng bridge traffic

| Family | Family → `្` | `្` → family |
|---|---:|---:|
| `ក` | 0.03488 | 0.00816 |
| `ន` | 0.02465 | 0.02204 |
| `ប` | 0.02727 | 0.00773 |
| `រ` | 0.00227 | 0.04042 |
| `ស` | 0.01068 | 0.00380 |
| `ច` | 0.00718 | 0.00400 |
| `ដ` | 0.00275 | 0.00297 |
