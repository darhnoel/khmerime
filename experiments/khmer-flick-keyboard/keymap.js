// Khmer flick keyboard — key map (MVP, editable)
// =================================================
// 15 keys in a 3x5 grid. Each key holds up to 5 glyphs reached by a flick:
//   c = center (quick tap), u = up, l = left, r = right, d = down.
// Empty string "" = no glyph in that direction.
//
// EDIT THIS FILE to correct any glyph — especially row 2 (independent vowels),
// which is a best-guess from the draft. The keyboard reads it live on reload.

// Row 1 — consonants, grouped by traditional series (velar, palatal, ...).
const ROW1 = [
  { c: 'ក', u: 'គ', l: 'ខ', r: 'ឃ', d: 'ង' },
  { c: 'ច', u: 'ជ', l: 'ឆ', r: 'ញ', d: 'ឈ' },
  { c: 'ដ', u: 'ឌ', l: 'ឋ', r: 'ឍ', d: 'ណ' },
  { c: 'ត', u: 'ទ', l: 'ថ', r: 'ធ', d: 'ន' },
  { c: 'ប', u: 'ព', l: 'ផ', r: 'ភ', d: 'ម' },
];

// Row 2 — remaining consonants + coeng + independent vowels. BEST GUESS — verify glyphs.
const ROW2 = [
  { c: 'យ', u: 'ល', l: 'រ', r: 'វ', d: 'ស' },
  { c: 'ហ', u: 'អ', l: 'ឡ', r: '',  d: '' },
  { c: '្', u: '', l: '', r: '', d: '' },
  { c: 'ឫ', u: 'ឭ', l: 'ឬ', r: 'ឮ', d: '' },
  { c: 'ឯ', u: 'ឱ', l: '',  r: 'ឲ', d: 'ឳ' },
];

// Row 3 — dependent vowels + diacritics + the ឥ vowel group. Key 4 holds the
// composed -ះ sras.
const ROW3 = [
  { c: 'ា', u: 'ី', l: 'ិ', r: 'ឹ', d: 'ឺ' },
  { c: 'ុ', u: 'ួ', l: 'ូ', r: 'ឿ', d: 'ៀ' },
  { c: 'េ', u: 'ៃ', l: 'ែ', r: 'ោ', d: 'ៅ' },
  { c: 'ំ', u: 'ុះ', l: 'ះ', r: 'េះ', d: 'ោះ' },
  { c: 'ឥ', u: 'ឧ', l: 'ឦ', r: 'ឩ', d: 'ឪ' },
];

const KEYMAP = [ROW1, ROW2, ROW3];

// Top-right legend grid, exactly as drawn in the draft.
const LEGEND = [
  ['ក', 'ច', 'ដ', 'ត', 'ប'],
  ['យ', 'ហ', '្', 'ឫ', 'ឯ'],
  ['ា', 'ុ', 'េ', 'ំ', 'ឥ'],
];
