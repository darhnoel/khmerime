#!/usr/bin/env node

import fs from 'node:fs';
import vm from 'node:vm';

const MODEL = Object.freeze({
  horizontalPitch: 1.12,
  verticalPitch: 1.05,
  thumbHome: Object.freeze([2, 2]),
  gravityCenter: Object.freeze([2, 1]),
  auxiliaryY: 3,
  reachWeight: 0.08,
  leanScale: 0.25,
  leanStep: 0.35,
});

const CORE_FAMILIES = Object.freeze([
  ['ើ', 'ៀ'],
  ['េ', 'ោ', 'ែ', 'ៅ', 'ៃ'],
  ['ុ', 'ូ', 'ួ', 'ឿ'],
  ['ា', 'ិ', 'ី', 'ឹ', 'ឺ'],
  ['ះ', 'ំ'],
  ['ដ', 'ណ', 'ឋ', 'ឌ', 'ឍ'],
  ['ក', 'ង', 'គ', 'ខ', 'ឃ'],
  ['រ', 'ល', 'យ', 'វ'],
  ['ន', 'ត', 'ទ', 'ធ', 'ថ'],
  ['ច', 'ជ', 'ញ', 'ឆ', 'ឈ'],
  ['ស', 'អ', 'ហ', 'ឡ'],
  ['្', '់', '។', '៉'],
  ['ប', 'ម', 'ព', 'ភ', 'ផ'],
]);

// These are exactly the 25 entries migrated from the former QUICK_ACCESS_ITEMS,
// grouped by linguistic/function relationship before position optimization.
const AUXILIARY_FAMILIES = Object.freeze([
  { id: 'independent-a', members: ['ឮ', 'ឫ', 'ឪ', 'ឭ', 'ឰ'] },
  { id: 'independent-b', members: ['ឲ្យ', 'ឦ', 'ឳ', 'ឩ', 'ឨ'] },
  { id: 'modifiers', members: ['៏', '័', '៍', '៌', '៊'] },
  { id: 'reading-signs', members: ['ៈ', 'ៗ', '៎', '៖', '៑'] },
  { id: 'terminal-symbols', members: ['៕', '៛', '៚', '៙', '៘'] },
]);

const DIRECTIONS = Object.freeze({
  u: [0, -1],
  l: [-1, 0],
  r: [1, 0],
  d: [0, 1],
});
const DIRECTION_COST = Object.freeze({ c: 0, u: 1.05, d: 1.05, l: 1.12, r: 1.12 });

function loadConst(file, name) {
  const context = {};
  vm.createContext(context);
  vm.runInContext(fs.readFileSync(new URL(file, import.meta.url), 'utf8'), context);
  return vm.runInContext(name, context);
}

function loadFrequency() {
  const text = fs.readFileSync(new URL('./char-frequency.txt', import.meta.url), 'utf8');
  const counts = new Map();
  let total = 0;
  for (const line of text.split('\n')) {
    const match = line.match(/^(\S+)\s+U\+[0-9A-F]+\s+(\d+)\s+/);
    if (!match) continue;
    const count = Number(match[2]);
    counts.set(match[1], count);
    total += count;
  }
  return new Map([...counts].map(([glyph, count]) => [glyph, count / total]));
}

function modelGlyph(member) {
  // The corpus is character-based. Use raw ឲ frequency as an explicit proxy for
  // the multi-character ឲ្យ shortcut until phrase-level shortcut data exists.
  return member === 'ឲ្យ' ? 'ឲ' : member;
}

function distance(a, b) {
  const dx = MODEL.horizontalPitch * (b[0] - a[0]);
  const dy = MODEL.verticalPitch * (b[1] - a[1]);
  return Math.hypot(dx, dy);
}

function permutations(items) {
  if (items.length < 2) return [items];
  return items.flatMap((item, index) =>
    permutations(items.filter((_, candidate) => candidate !== index))
      .map(rest => [item, ...rest]));
}

function inwardDirections(x) {
  return Object.entries(DIRECTIONS)
    .map(([direction, [dx, dy]]) => ({
      direction,
      distance: distance(
        [x + dx * MODEL.leanStep, MODEL.auxiliaryY + dy * MODEL.leanStep],
        MODEL.gravityCenter,
      ),
    }))
    .sort((a, b) => a.distance - b.distance || DIRECTION_COST[a.direction] - DIRECTION_COST[b.direction])
    .map(candidate => candidate.direction);
}

function arrangeFamily(family, x, unigram) {
  const ranked = [...family.members].sort((a, b) =>
    (unigram.get(modelGlyph(b)) || 0) - (unigram.get(modelGlyph(a)) || 0));
  const mapping = { c: ranked[0] };
  inwardDirections(x).forEach((direction, index) => { mapping[direction] = ranked[index + 1]; });
  return mapping;
}

function corePositions(keymap) {
  const positions = new Map();
  for (const glyphs of CORE_FAMILIES) {
    const match = keymap.flatMap((row, y) => row.map((key, x) => ({ key, x, y })))
      .find(({ key }) => glyphs.some(glyph => Object.values(key).includes(glyph)));
    if (!match) throw new Error(`Missing core family ${glyphs[0]}`);
    for (const glyph of glyphs) positions.set(glyph, [match.x, match.y]);
  }
  return positions;
}

function analyze(order, keymap, unigram, bigram) {
  const positions = corePositions(keymap);
  const auxiliaryMappings = [];
  order.forEach((family, x) => {
    const mapping = arrangeFamily(family, x, unigram);
    auxiliaryMappings.push({ x, family, mapping });
    for (const member of Object.values(mapping)) positions.set(modelGlyph(member), [x, MODEL.auxiliaryY]);
  });

  let transitionMass = 0;
  let transitionCost = 0;
  for (const [from, next] of Object.entries(bigram)) {
    const fromPosition = positions.get(from);
    const pFrom = unigram.get(from) || 0;
    if (!fromPosition || !pFrom) continue;
    for (const [to, conditional] of Object.entries(next)) {
      const toPosition = positions.get(to);
      if (!toPosition) continue;
      const mass = pFrom * conditional;
      transitionMass += mass;
      transitionCost += mass * distance(fromPosition, toPosition);
    }
  }
  const transitionTravel = transitionCost / transitionMass;

  const uniqueGlyphs = new Set(positions.keys());
  let reachMass = 0;
  let reachCost = 0;
  for (const glyph of uniqueGlyphs) {
    const p = unigram.get(glyph) || 0;
    reachMass += p;
    reachCost += p * distance(MODEL.thumbHome, positions.get(glyph));
  }
  const reach = reachCost / reachMass;
  const effort = transitionTravel + MODEL.reachWeight * reach;

  let selectionMass = 0;
  let selectionCost = 0;
  for (const row of keymap.slice(0, 3)) {
    for (const key of row) {
      for (const [direction, glyph] of Object.entries(key)) {
        if (!glyph || Array.from(glyph).length !== 1 || !uniqueGlyphs.has(glyph)) continue;
        const p = unigram.get(glyph) || 0;
        selectionMass += p;
        selectionCost += p * DIRECTION_COST[direction];
      }
    }
  }
  for (const { mapping } of auxiliaryMappings) {
    for (const [direction, member] of Object.entries(mapping)) {
      const p = unigram.get(modelGlyph(member)) || 0;
      selectionMass += p;
      selectionCost += p * DIRECTION_COST[direction];
    }
  }
  const selection = selectionCost / selectionMass;
  const combined = effort + MODEL.leanScale * selection;
  return { auxiliaryMappings, transitionMass, transitionTravel, reach, effort, selection, combined };
}

const keymap = loadConst('./keymap.js', 'KEYMAP');
const bigram = loadConst('./ngram.js', 'BIGRAM');
const unigram = loadFrequency();
const baseline = analyze([], keymap, unigram, bigram);
const candidates = permutations(AUXILIARY_FAMILIES)
  .map(order => analyze(order, keymap, unigram, bigram))
  .sort((a, b) => a.combined - b.combined);
const best = candidates[0];
const bestEffort = [...candidates].sort((a, b) => a.effort - b.effort)[0];

console.log(`Candidates searched: ${candidates.length}`);
console.log(`Assumptions: auxiliaryY=${MODEL.auxiliaryY}, gravityCenter=(${MODEL.gravityCenter}), leanScale=${MODEL.leanScale}`);
console.log(`Comparable core E: ${baseline.effort.toFixed(8)}; core E + lean: ${baseline.combined.toFixed(8)}`);
for (const { x, family, mapping } of best.auxiliaryMappings) {
  console.log(`x=${x} ${family.id}: c=${mapping.c} u=${mapping.u} l=${mapping.l} r=${mapping.r} d=${mapping.d}`);
}
console.log(`Transition mass:   ${best.transitionMass.toFixed(8)}`);
console.log(`Transition travel: ${best.transitionTravel.toFixed(8)}`);
console.log(`Reach:             ${best.reach.toFixed(8)}`);
console.log(`Effort:            ${best.effort.toFixed(8)}`);
console.log(`Selection score:   ${best.selection.toFixed(8)}`);
console.log(`Effort + lean:     ${best.combined.toFixed(8)}`);
console.log(`Best pure-E order: ${bestEffort.auxiliaryMappings.map(item => item.family.id).join(' | ')}`);
console.log(`Best pure E:       ${bestEffort.effort.toFixed(8)}; E + lean: ${bestEffort.combined.toFixed(8)}`);
