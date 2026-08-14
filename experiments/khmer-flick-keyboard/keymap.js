// Khmer flick keyboard — key map (frequency-optimized symmetric layout)
// ====================================================================
// 15 keys in a 3x5 grid. Each key holds up to 5 glyphs reached by a flick:
//   c = center (quick tap), u = up, l = left, r = right, d = down.
//
// Structure (backed by corpus statistics on 31.4M Khmer chars):
//   - COENG ្ at the CENTER (row2 col3) — it is 100% consonant-adjacent in text.
//   - CONSONANTS ring the centre (the 3 middle columns): 5 varga series + liquids + sibilant.
//   - VOWELS on the two OUTER columns.
//   - Within every group the MOST FREQUENT glyph is the center tap; rarer ones are flicks.
//   - Combining diacritics (៉ ់ ៊ ័ ...) live in the Quick Access tray, not the grid.

const ROW1 = [
  { c: 'ា', u: 'ិ', l: 'ី', r: 'ឹ', d: 'ឺ' },
  { c: 'ក', u: 'ង', l: 'គ', r: 'ខ', d: 'ឃ' },
  { c: 'ន', u: 'ត', l: 'ទ', r: 'ធ', d: 'ថ' },
  { c: 'ប', u: 'ម', l: 'ព', r: 'ភ', d: 'ផ' },
  { c: 'ះ', u: 'ំ', l: '', r: '', d: '' }
];

const ROW2 = [
  { c: 'េ', u: 'ោ', l: 'ែ', r: 'ៅ', d: 'ៃ' },
  { c: 'រ', u: 'ល', l: 'យ', r: 'វ', d: '' },
  { c: '្', u: '', l: '', r: '', d: '' },
  { c: 'ស', u: 'អ', l: 'ហ', r: 'ឡ', d: '' },
  { c: 'ើ', u: 'ៀ', l: '', r: '', d: '' }
];

const ROW3 = [
  { c: 'ុ', u: 'ូ', l: 'ួ', r: 'ឿ', d: '' },
  { c: 'ច', u: 'ជ', l: 'ញ', r: 'ឆ', d: 'ឈ' },
  { c: 'ដ', u: 'ណ', l: 'ឋ', r: 'ឌ', d: 'ឍ' },
  { c: 'ឱ', u: 'ឯ', l: 'ឧ', r: 'ឬ', d: 'ឥ' },
  { c: 'ុះ', u: 'េះ', l: 'ោះ', r: '', d: '' }
];

const KEYMAP = [ROW1, ROW2, ROW3];

// Legend grid (top-right): the 15 centre glyphs, row-major.
const LEGEND = [
  ['ា', 'ក', 'ន', 'ប', 'ះ'],
  ['េ', 'រ', '្', 'ស', 'ើ'],
  ['ុ', 'ច', 'ដ', 'ឯ', 'ុះ'],
];
