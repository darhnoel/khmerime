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

function parseArgs(args) {
  const options = { swap: null, centers: 'frequency', leanScale: 0 };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--swap' && args[i + 1] && args[i + 2]) {
      options.swap = [args[++i], args[++i]];
    } else if (args[i] === '--centers' && ['frequency', 'yt'].includes(args[i + 1])) {
      options.centers = args[++i];
    } else if (args[i] === '--lean-scale' && Number.isFinite(Number(args[i + 1])) && Number(args[i + 1]) >= 0) {
      options.leanScale = Number(args[++i]);
    } else {
      throw new Error('Usage: node analyze-layout.mjs [--swap A B] [--centers frequency|yt] [--lean-scale NUMBER]');
    }
  }
  return options;
}

function applySwap(positions, pair) {
  if (!pair) return null;
  const first = resolveFamily(pair[0]);
  const second = resolveFamily(pair[1]);
  const firstPosition = positions.get(first);
  positions.set(first, positions.get(second));
  positions.set(second, firstPosition);
  return [first, second];
}

function applyCenterVariant(keymap, variant) {
  if (variant === 'frequency') return keymap;
  return keymap.map(row => row.map(original => {
    const key = { ...original };
    if (key.c === 'រ') [key.c, key.l] = [key.l, key.c];
    if (key.c === 'ន') [key.c, key.u] = [key.u, key.c];
    return key;
  }));
}

function analyzeSelection(unigram, keymap) {
  const modeled = new Set(FAMILIES.flatMap(family => family.glyphs));
  const directionCost = { c: 0, u: 1.05, d: 1.05, l: 1.12, r: 1.12 };
  let mass = 0;
  let cost = 0;
  for (const row of keymap) {
    for (const key of row) {
      for (const [direction, glyph] of Object.entries(key)) {
        if (!glyph || !modeled.has(glyph)) continue;
        const probability = unigram[glyph] ?? 0;
        mass += probability;
        cost += probability * directionCost[direction];
      }
    }
  }
  return { mass, cost, score: cost / mass };
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

const options = parseArgs(process.argv.slice(2));
const keymap = applyCenterVariant(loadConst('./keymap.js', 'KEYMAP'), options.centers);
const unigram = loadConst('./ngram.js', 'UNIGRAM');
const bigram = loadConst('./ngram.js', 'BIGRAM');
const positions = findPositions(keymap);
const swapped = applySwap(positions, options.swap);
const result = analyze(unigram, bigram, positions);
const selection = analyzeSelection(unigram, keymap);
const effortWithLean = result.effort + options.leanScale * selection.score;

const layoutParts = [options.centers === 'yt' ? 'centers យ/ត' : 'frequency centers'];
if (swapped) layoutParts.push(`swap ${swapped[0]} ↔ ${swapped[1]}`);
console.log(`Layout: ${layoutParts.join(', ')}`);
console.log(`Assumptions: horizontal=${MODEL.horizontalPitch}, vertical=${MODEL.verticalPitch}, thumbHome=(${MODEL.thumbHome.join(',')}), reachWeight=${MODEL.reachWeight}`);
console.log(`Transition mass:   ${result.transitionMass.toFixed(8)}`);
console.log(`Transition cost:   ${result.transitionCost.toFixed(8)}`);
console.log(`Transition travel: ${result.transitionTravel.toFixed(8)}`);
console.log(`Reach mass:        ${result.reachMass.toFixed(8)}`);
console.log(`Reach cost:        ${result.reachCost.toFixed(8)}`);
console.log(`Reach:             ${result.reach.toFixed(8)}`);
console.log(`Effort:            ${result.effort.toFixed(8)}`);
console.log(`Selection mass:    ${selection.mass.toFixed(8)}`);
console.log(`Selection cost:    ${selection.cost.toFixed(8)}`);
console.log(`Selection score:   ${selection.score.toFixed(8)}`);
console.log(`Lean scale:        ${options.leanScale.toFixed(8)}`);
console.log(`Effort + lean:     ${effortWithLean.toFixed(8)}`);
