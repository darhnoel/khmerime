// Khmer flick keyboard — transition-optimized one-thumb layout
// ============================================================
// 15 keys in a 3x5 grid. Each key holds up to 5 glyphs reached by a flick:
//   c = center (quick tap), u = up, l = left, r = right, d = down.
//
// Structure (backed by bigram transitions from 31.4M Khmer chars):
//   - VOWEL / ENDING families occupy row 1: ើ េ ុ ា ះ.
//   - High-traffic consonant families occupy row 2: ដ ក រ ន ច.
//   - COENG ្ bridges ស and ប at the thumb's lower centre, with three former Quick Access marks.
//   - Rare independent/composed groups occupy the two lower corners.
//   - Within every group the MOST FREQUENT glyph is the center tap; rarer ones are flicks.
//   - Bantoc ់, muusikatoan ៉, and khan ។ are coeng flicks; rarer marks stay in Quick Access.

const ROW1 = [
  { c: 'ើ', u: '', l: '', r: '', d: 'ៀ' },
  { c: 'េ', u: 'ៅ', l: 'ៃ', r: 'ែ', d: 'ោ' },
  { c: 'ុ', u: '', l: 'ឿ', r: 'ួ', d: 'ូ' },
  { c: 'ា', u: 'ឹ', l: 'ី', r: 'ឺ', d: 'ិ' },
  { c: 'ះ', u: '', l: 'ំ', r: '', d: '' }
];

const ROW2 = [
  { c: 'ដ', u: 'ណ', l: 'ឍ', r: 'ឌ', d: 'ឋ' },
  { c: 'ក', u: 'ង', l: 'ឃ', r: 'ខ', d: 'គ' },
  { c: 'រ', u: 'ល', l: 'យ', r: 'វ', d: '' },
  { c: 'ន', u: 'ត', l: 'ធ', r: 'ថ', d: 'ទ' },
  { c: 'ច', u: 'ជ', l: 'ឆ', r: 'ឈ', d: 'ញ' }
];

const ROW3 = [
  { c: 'ឯ', u: 'ឧ', l: 'ឱ', r: 'ឬ', d: 'ឥ' },
  { c: 'ស', u: 'អ', l: 'ឡ', r: 'ហ', d: '' },
  { c: '្', u: '់', l: '៉', r: '។', d: '' },
  { c: 'ប', u: 'ម', l: 'ព', r: 'ផ', d: 'ភ' },
  { c: 'ោះ', u: 'េះ', l: 'ុះ', r: '', d: '' }
];

const KEYMAP = [ROW1, ROW2, ROW3];

// Legend grid (top-right): the 15 centre glyphs, row-major.
const LEGEND = [
  ['ើ', 'េ', 'ុ', 'ា', 'ះ'],
  ['ដ', 'ក', 'រ', 'ន', 'ច'],
  ['ឯ', 'ស', '្', 'ប', 'ោះ'],
];
