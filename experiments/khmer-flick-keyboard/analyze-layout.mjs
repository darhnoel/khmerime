#!/usr/bin/env node

import fs from 'node:fs';
import vm from 'node:vm';

const MODEL = Object.freeze({
  horizontalPitch: 1.12,
  verticalPitch: 1.05,
  thumbHome: Object.freeze([2, 2]),
  reachWeight: 0.08,
});

// Rare independent-vowel and composed-entry corner families are deliberately
// excluded: the pruned UNIGRAM model has no probabilities for the former, and
// the latter are multi-code-point gestures rather than n-gram tokens.
const FAMILIES = Object.freeze([
  { id: 'oe', glyphs: ['ើ', 'ៀ'] },
  { id: 'e', glyphs: ['េ', 'ោ', 'ែ', 'ៅ', 'ៃ'] },
  { id: 'u', glyphs: ['ុ', 'ូ', 'ួ', 'ឿ'] },
  { id: 'aa', glyphs: ['ា', 'ិ', 'ី', 'ឹ', 'ឺ'] },
  { id: 'ending', glyphs: ['ះ', 'ំ'] },
  { id: 'da', glyphs: ['ដ', 'ណ', 'ឋ', 'ឌ', 'ឍ'] },
  { id: 'ka', glyphs: ['ក', 'ង', 'គ', 'ខ', 'ឃ'] },
  { id: 'ro', glyphs: ['រ', 'ល', 'យ', 'វ'] },
  { id: 'no', glyphs: ['ន', 'ត', 'ទ', 'ធ', 'ថ'] },
  { id: 'ca', glyphs: ['ច', 'ជ', 'ញ', 'ឆ', 'ឈ'] },
  { id: 'sa', glyphs: ['ស', 'អ', 'ហ', 'ឡ'] },
  { id: 'coeng', glyphs: ['្', '់', '។', '៉'] },
  { id: 'ba', glyphs: ['ប', 'ម', 'ព', 'ភ', 'ផ'] },
]);

function loadConst(file, name) {
  const context = {};
  vm.createContext(context);
  vm.runInContext(fs.readFileSync(new URL(file, import.meta.url), 'utf8'), context);
  return vm.runInContext(name, context);
}

function findPositions(keymap) {
  const positions = new Map();

  for (const family of FAMILIES) {
    const matches = [];
    keymap.forEach((row, y) => row.forEach((key, x) => {
      const entries = Object.values(key).filter(Boolean);
      if (family.glyphs.some(glyph => entries.includes(glyph))) matches.push([x, y]);
    }));

    const unique = [...new Map(matches.map(point => [point.join(','), point])).values()];
    if (unique.length !== 1) {
      throw new Error(`${family.id}: expected one key position, found ${unique.length}`);
    }
    positions.set(family.id, unique[0]);
  }

  return positions;
}

function distance(a, b) {
  const dx = MODEL.horizontalPitch * (b[0] - a[0]);
  const dy = MODEL.verticalPitch * (b[1] - a[1]);
  return Math.hypot(dx, dy);
}

function resolveFamily(value) {
  const family = FAMILIES.find(candidate =>
    candidate.id === value || candidate.glyphs.includes(value));
  if (!family) throw new Error(`Unknown family or glyph: ${value}`);
  return family.id;
}

function applySwap(positions, args) {
  if (args.length === 0) return null;
  if (args.length !== 3 || args[0] !== '--swap') {
    throw new Error('Usage: node analyze-layout.mjs [--swap FAMILY_OR_GLYPH FAMILY_OR_GLYPH]');
  }

  const first = resolveFamily(args[1]);
  const second = resolveFamily(args[2]);
  const firstPosition = positions.get(first);
  positions.set(first, positions.get(second));
  positions.set(second, firstPosition);
  return [first, second];
}

function analyze(unigram, bigram, positions) {
  const owner = new Map();
  for (const family of FAMILIES) {
    for (const glyph of family.glyphs) owner.set(glyph, family.id);
  }

  let transitionMass = 0;
  let transitionCost = 0;
  for (const [from, probabilityFrom] of Object.entries(unigram)) {
    const fromFamily = owner.get(from);
    if (!fromFamily || !bigram[from]) continue;

    for (const [to, probabilityToGivenFrom] of Object.entries(bigram[from])) {
      const toFamily = owner.get(to);
      if (!toFamily) continue;

      const jointProbability = probabilityFrom * probabilityToGivenFrom;
      transitionMass += jointProbability;
      transitionCost += jointProbability * distance(
        positions.get(fromFamily), positions.get(toFamily));
    }
  }

  let reachMass = 0;
  let reachCost = 0;
  for (const family of FAMILIES) {
    const familyMass = family.glyphs.reduce(
      (sum, glyph) => sum + (unigram[glyph] ?? 0), 0);
    reachMass += familyMass;
    reachCost += familyMass * distance(positions.get(family.id), MODEL.thumbHome);
  }

  const transitionTravel = transitionCost / transitionMass;
  const reach = reachCost / reachMass;
  const effort = transitionTravel + MODEL.reachWeight * reach;

  return { transitionMass, transitionCost, transitionTravel, reachMass, reachCost, reach, effort };
}

const keymap = loadConst('./keymap.js', 'KEYMAP');
const unigram = loadConst('./ngram.js', 'UNIGRAM');
const bigram = loadConst('./ngram.js', 'BIGRAM');
const positions = findPositions(keymap);
const swapped = applySwap(positions, process.argv.slice(2));
const result = analyze(unigram, bigram, positions);

console.log(swapped ? `Layout: swap ${swapped[0]} ↔ ${swapped[1]}` : 'Layout: current keymap');
console.log(`Assumptions: horizontal=${MODEL.horizontalPitch}, vertical=${MODEL.verticalPitch}, thumbHome=(${MODEL.thumbHome.join(',')}), reachWeight=${MODEL.reachWeight}`);
console.log(`Transition mass:   ${result.transitionMass.toFixed(8)}`);
console.log(`Transition cost:   ${result.transitionCost.toFixed(8)}`);
console.log(`Transition travel: ${result.transitionTravel.toFixed(8)}`);
console.log(`Reach mass:        ${result.reachMass.toFixed(8)}`);
console.log(`Reach cost:        ${result.reachCost.toFixed(8)}`);
console.log(`Reach:             ${result.reach.toFixed(8)}`);
console.log(`Effort:            ${result.effort.toFixed(8)}`);
