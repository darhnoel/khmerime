// Khmer Directional Lean keyboard — transition-optimized one-thumb layout
// ============================================================
// 20 keys in a 4x5 grid. Each key holds up to 5 entries reached by a lean:
//   c = center (quick tap), u = up, l = left, r = right, d = down.
//
// Structure (backed by bigram transitions from 31.4M Khmer chars):
//   - VOWEL / ENDING families occupy row 1: ើ េ ុ ា ះ.
//   - High-traffic consonant families occupy row 2: ដ ក រ ន ច.
//   - COENG ្ bridges ស and ប at the thumb's lower centre, with three former Quick Access marks.
//   - Rare independent/composed groups occupy the two lower core corners.
//   - Within every group the MOST FREQUENT entry is the center tap; rarer ones are leans.
//   - Bantoc ់, muusikatoan ៉, and khan ។ are coeng direction members.
//   - The bottom Auxiliary Family Row replaces Quick Access with five fixed families.

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

const AUXILIARY_ROW = [
  { c: '៕', u: '៚', l: '៘', r: '៛', d: '៙' },
  { c: 'ឲ្យ', u: 'ឦ', l: 'ឩ', r: 'ឳ', d: 'ឨ' },
  { c: '៏', u: '័', l: '៍', r: '៌', d: '៊' },
  { c: 'ៈ', u: 'ៗ', l: '៎', r: '៖', d: '៑' },
  { c: 'ឮ', u: 'ឪ', l: 'ឫ', r: 'ឰ', d: 'ឭ' }
];

const KEYMAP = [ROW1, ROW2, ROW3, AUXILIARY_ROW];

// Legend grid: the 20 centre entries, row-major.
const LEGEND = [
  ['ើ', 'េ', 'ុ', 'ា', 'ះ'],
  ['ដ', 'ក', 'រ', 'ន', 'ច'],
  ['ឯ', 'ស', '្', 'ប', 'ោះ'],
  ['៕', 'ឲ្យ', '៏', 'ៈ', 'ឮ'],
];
