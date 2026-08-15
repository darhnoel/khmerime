// Khmer flick keyboard — transition-optimized one-thumb layout
// ============================================================
// 15 keys in a 3x5 grid. Each key holds up to 5 glyphs reached by a flick:
//   c = center (quick tap), u = up, l = left, r = right, d = down.
//
// Structure (backed by bigram transitions from 31.4M Khmer chars):
//   - VOWEL / ENDING families occupy row 1: ើ េ ុ ា ះ.
//   - High-traffic consonant families occupy row 2: ដ ក រ ន ច.
//   - COENG ្ bridges ស and ប at the thumb's lower centre.
//   - Rare independent/composed groups occupy the two lower corners.
//   - Within every group the MOST FREQUENT glyph is the center tap; rarer ones are flicks.
//   - Combining diacritics (៉ ់ ៊ ័ ...) live in the Quick Access tray, not the grid.

const ROW1 = [
  { c: 'ើ', u: '', l: '', r: '', d: 'ៀ' },
  { c: 'េ', u: 'ោ', l: 'ៃ', r: 'ៅ', d: 'ែ' },
  { c: 'ុ', u: 'ូ', l: 'ួ', r: 'ឿ', d: '' },
  { c: 'ា', u: 'ិ', l: 'ី', r: 'ឹ', d: 'ឺ' },
  { c: 'ះ', u: 'ំ', l: '', r: '', d: '' }
];

const ROW2 = [
  { c: 'ដ', u: 'ណ', l: 'ឋ', r: 'ឌ', d: 'ឍ' },
  { c: 'ក', u: 'ង', l: 'គ', r: 'ខ', d: 'ឃ' },
  { c: 'រ', u: 'ល', l: 'យ', r: 'វ', d: '' },
  { c: 'ន', u: 'ត', l: 'ទ', r: 'ធ', d: 'ថ' },
  { c: 'ច', u: 'ជ', l: 'ញ', r: 'ឆ', d: 'ឈ' }
];

const ROW3 = [
  { c: 'ឱ', u: 'ឯ', l: 'ឧ', r: 'ឬ', d: 'ឥ' },
  { c: 'ស', u: 'អ', l: 'ហ', r: 'ឡ', d: '' },
  { c: '្', u: '', l: '', r: '', d: '' },
  { c: 'ប', u: 'ម', l: 'ព', r: 'ភ', d: 'ផ' },
  { c: 'ុះ', u: 'េះ', l: 'ោះ', r: '', d: '' }
];

const KEYMAP = [ROW1, ROW2, ROW3];

// Legend grid (top-right): the 15 centre glyphs, row-major.
const LEGEND = [
  ['ើ', 'េ', 'ុ', 'ា', 'ះ'],
  ['ដ', 'ក', 'រ', 'ន', 'ច'],
  ['ឱ', 'ស', '្', 'ប', 'ុះ'],
];
