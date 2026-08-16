// Khmer compact keyboard — Directional Lean interaction (MVP)
// ===========================================================
// Pressing previews the center member. A subtle shift toward a displayed family
// member updates the preview; releasing commits it. Returning to neutral restores
// the center. Works with touch and mouse (mouse = desktop testing).

const output = document.getElementById('output');
const copyOutputButton = document.getElementById('copy-output');
const popup = document.getElementById('popup');
const kb = document.getElementById('kb');
const quickAccess = document.getElementById('quick-access');
const previewMembers = Object.fromEntries(
  ['c', 'u', 'l', 'r', 'd'].map(dir => [dir, popup.querySelector(`.preview-member.${dir}`)])
);
const nightToggle = document.getElementById('night-toggle');
const heatToggle = document.getElementById('heat-toggle');
const centerToggle = document.getElementById('center-toggle');
const layoutToggle = document.getElementById('layout-toggle');

const LEAN_THRESHOLD = 10; // px around initial contact that keeps the center selected
const PREVIEW_GAP = 18;    // px between the finger and preview bubble

let active = null; // { def, key, pointerId, startX, startY, dir }

copyOutputButton.addEventListener('click', copyOutputText);
nightToggle.addEventListener('click', () => setDisplayOption('night', !document.body.classList.contains('night')));
heatToggle.addEventListener('click', () => setDisplayOption('heat', document.body.classList.contains('heat-off')));
centerToggle.addEventListener('click', () => setDisplayOption('center', !document.body.classList.contains('center-only')));
layoutToggle.addEventListener('click', () => {
  const enabled = !document.body.classList.contains('layout-b');
  setDisplayOption('layout', enabled);
  cancelLean();
  renderKeymap();
  updateHeat();
});

function savedDisplayOption(name, fallback) {
  try {
    const value = localStorage.getItem(`khmer-lean-${name}`);
    return value === null ? fallback : value === 'true';
  } catch {
    return fallback;
  }
}

function setDisplayOption(name, enabled, persist = true) {
  if (name === 'night') {
    document.body.classList.toggle('night', enabled);
    nightToggle.setAttribute('aria-checked', String(enabled));
    document.querySelector('meta[name="theme-color"]').content = enabled ? '#181b20' : '#f6f7f9';
  } else if (name === 'heat') {
    document.body.classList.toggle('heat-off', !enabled);
    heatToggle.setAttribute('aria-checked', String(enabled));
  } else if (name === 'center') {
    document.body.classList.toggle('center-only', enabled);
    centerToggle.setAttribute('aria-checked', String(enabled));
  } else {
    document.body.classList.toggle('layout-b', enabled);
    layoutToggle.setAttribute('aria-checked', String(enabled));
  }
  if (persist) {
    try { localStorage.setItem(`khmer-lean-${name}`, String(enabled)); } catch { /* storage is optional */ }
  }
}

setDisplayOption('night', savedDisplayOption('night', false), false);
setDisplayOption('heat', savedDisplayOption('heat', true), false);
setDisplayOption('center', savedDisplayOption('center', false), false);
setDisplayOption('layout', savedDisplayOption('layout', false), false);

async function copyOutputText() {
  const text = committed + preedit;
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const selection = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(output);
    selection.removeAllRanges();
    selection.addRange(range);
    document.execCommand('copy');
    selection.removeAllRanges();
  }
  copyOutputButton.textContent = 'បានចម្លង';
  setTimeout(() => { copyOutputButton.textContent = 'ចម្លង'; }, 1200);
}

// Matches the Android/iOS QuickAccessSpec ordering. Dotted circles are
// display-only in the browser; insertion always uses the raw Khmer mark.
// Ordered by corpus frequency (most-used first), measured on the kmwiki corpus.
const QUICK_ACCESS_ITEMS = [
  { display: 'ឲ្យ', commit: 'ឲ្យ', label: 'Aoy (to give / let)' },                 // common word
  { display: '៏', commit: '៏', label: 'Ahsda' },                                  // 0.33%
  { display: '័', commit: '័', label: 'Samyok sannya' },                          // 0.27%
  { display: 'ៈ', commit: 'ៈ', label: 'Yukaleapintu' },                           // 0.23%
  { display: '៍', commit: '៍', label: 'Toandakhiat' },                            // 0.14%
  { display: '៌', commit: '៌', label: 'Robat' },                                  // 0.14%
  { display: '៊', commit: '៊', label: 'Triisap' },                               // 0.08%
  { display: 'ៗ', commit: 'ៗ', label: 'Khmer repetition sign' },                  // 0.07%
  { display: '៎', commit: '៎', label: 'Kakabat' },                               // 0.02%
  { display: '៖', commit: '៖', label: 'Khmer sign camnuc pii kuuh' },             // 0.015%
  { display: '៑', commit: '៑', label: 'Viriam' },                                // rare
  { display: '៕', commit: '៕', label: 'Khmer final period' },                     // rare
  { display: '៛', commit: '៛', label: 'Khmer currency symbol riel' },             // rare
  { display: '៚', commit: '៚', label: 'Koomuut' },                                // rare
  { display: '៙', commit: '៙', label: 'Phnaek muan' },                            // rare
  { display: '៘', commit: '៘', label: 'Beyyal' },                                 // rare
  // Rare independent vowels (freq-ordered) — reachable here rather than on the grid.
  { display: 'ឮ', commit: 'ឮ', label: 'Independent vowel LYY' },
  { display: 'ឫ', commit: 'ឫ', label: 'Independent vowel RY' },
  { display: 'ឪ', commit: 'ឪ', label: 'Independent vowel QUUV' },
  { display: 'ឭ', commit: 'ឭ', label: 'Independent vowel LY' },
  { display: 'ឰ', commit: 'ឰ', label: 'Independent vowel QAI' },
  { display: 'ឦ', commit: 'ឦ', label: 'Independent vowel QII' },
  { display: 'ឳ', commit: 'ឳ', label: 'Independent vowel QAU' },
  { display: 'ឩ', commit: 'ឩ', label: 'Independent vowel QUU' },
  { display: 'ឨ', commit: 'ឨ', label: 'Independent vowel QUK' },
];

// --- Build the keyboard from KEYMAP ------------------------------------------
const keyEls = []; // all character-key elements, for the bigram heatmap
const keyRowEls = KEYMAP.map(() => {
  const rowEl = document.createElement('div');
  rowEl.className = 'kb-row';
  kb.appendChild(rowEl);
  return rowEl;
});
renderKeymap();

QUICK_ACCESS_ITEMS.forEach(item => quickAccess.appendChild(makeQuickAccessKey(item)));

// Familiar mobile action row. Non-character controls are deliberately inert in
// this layout prototype, but retain pressed feedback so the keyboard feels real.
const actions = document.createElement('div');
actions.className = 'kb-row actions';
actions.appendChild(makeAction('123', () => {}, 'mode', 'Numbers'));
actions.appendChild(makeAction('ដកឃ្លា', space, 'space', 'Space'));
actions.appendChild(makeAction('⏎', commitComposition, 'return', 'Return'));
actions.appendChild(makeBackspace());
kb.appendChild(actions);

function makeKey(def) {
  const el = document.createElement('button');
  el.type = 'button';
  el.className = 'key';
  el.setAttribute('aria-label', def.c);
  const center = document.createElement('span');
  center.className = 'center member';
  center.textContent = def.c;
  el.appendChild(center);
  el.__members = { c: center };
  for (const dir of ['u', 'l', 'r', 'd']) {
    if (def[dir]) {
      const h = document.createElement('span');
      h.className = `hint ${dir}`;
      const member = document.createElement('span');
      member.className = 'member';
      member.textContent = def[dir];
      h.appendChild(member);
      el.appendChild(h);
      el.__members[dir] = member;
    }
  }
  el.__def = def;
  el.addEventListener('pointerdown', onPress);
  return el;
}

function activeKeymap() {
  if (!document.body.classList.contains('layout-b')) return KEYMAP;
  return KEYMAP.map(row => row.map(original => {
    const def = { ...original };
    if (def.c === 'រ') [def.c, def.l] = [def.l, def.c];
    if (def.c === 'ន') [def.c, def.u] = [def.u, def.c];
    return def;
  }));
}

function renderKeymap() {
  keyEls.length = 0;
  activeKeymap().forEach((row, rowIndex) => {
    const children = row.map(def => {
      const el = makeKey(def);
      keyEls.push(el);
      return el;
    });
    keyRowEls[rowIndex].replaceChildren(...children);
  });
}

function makeAction(label, fn, className = '', ariaLabel = label) {
  const el = document.createElement('button');
  el.type = 'button';
  el.className = `action ${className}`;
  el.setAttribute('aria-label', ariaLabel);
  el.textContent = label;
  el.addEventListener('pointerdown', e => { e.preventDefault(); fn(); });
  return el;
}

// Backspace with hold-to-repeat: one delete on press, then accelerating repeats
// while held (500 ms initial delay, then every 60 ms), stopping on release.
function makeBackspace() {
  const el = document.createElement('button');
  el.type = 'button';
  el.className = 'action icon';
  el.setAttribute('aria-label', 'Delete');
  el.textContent = '⌫';
  let repeatTimer = null, delayTimer = null;
  const stop = () => { clearTimeout(delayTimer); clearInterval(repeatTimer); delayTimer = repeatTimer = null; };
  el.addEventListener('pointerdown', e => {
    e.preventDefault();
    backspace();                                   // immediate first delete
    delayTimer = setTimeout(() => {
      repeatTimer = setInterval(backspace, 60);    // then repeat while held
    }, 500);
  });
  for (const ev of ['pointerup', 'pointerleave', 'pointercancel']) el.addEventListener(ev, stop);
  return el;
}

function makeQuickAccessKey(item) {
  const el = document.createElement('button');
  el.type = 'button';
  el.className = 'quick-key';
  el.textContent = item.display;
  el.setAttribute('aria-label', item.label || item.commit);
  // Click, rather than pointerdown, lets a horizontal swipe scroll the tray
  // without accidentally inserting the sign under the initial touch point.
  el.addEventListener('click', () => insert(item.commit));
  return el;
}

// --- Gesture handling ---------------------------------------------------------
function onPress(e) {
  e.preventDefault();
  if (active) return;
  e.currentTarget.setPointerCapture?.(e.pointerId);
  const def = e.currentTarget.__def;
  active = {
    def,
    key: e.currentTarget,
    pointerId: e.pointerId,
    startX: e.clientX,
    startY: e.clientY,
    dir: 'c',
  };
  active.key.classList.add('is-pressed');
  showPreview(def, 'c', e.clientX, e.clientY);
  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onRelease, { once: true });
  window.addEventListener('pointercancel', onCancel, { once: true });
}

function onMove(e) {
  if (!active || e.pointerId !== active.pointerId) return;
  const dir = directionOf(e.clientX - active.startX, e.clientY - active.startY, active.def);
  active.dir = dir;
  updatePreview(dir, e.clientX, e.clientY);
}

function onRelease(e) {
  if (active && e.pointerId !== active.pointerId) return;
  window.removeEventListener('pointermove', onMove);
  window.removeEventListener('pointercancel', onCancel);
  if (active) {
    // Movement only previews. Make the actual choice once, using the final
    // release vector, so a fast flick cannot leave a stale direction selected.
    const dir = directionOf(e.clientX - active.startX, e.clientY - active.startY, active.def);
    const glyph = active.def[dir];
    if (glyph) insert(glyph);
    active.key.classList.remove('is-pressed');
  }
  hidePopup();
  active = null;
}

function onCancel(e) {
  if (active && e.pointerId !== active.pointerId) return;
  window.removeEventListener('pointermove', onMove);
  window.removeEventListener('pointerup', onRelease);
  cancelLean();
}

function cancelLean() {
  window.removeEventListener('pointermove', onMove);
  active?.key.classList.remove('is-pressed');
  hidePopup();
  active = null;
}

// A displacement only selects a direction after leaving the neutral zone.
// Diagonals use their dominant axis. Missing family members fall back to center.
function directionOf(dx, dy, def) {
  const dist = Math.hypot(dx, dy);
  if (dist < LEAN_THRESHOLD) return 'c';
  const horizontal = Math.abs(dx) > Math.abs(dy);
  let dir;
  if (horizontal) dir = dx > 0 ? 'r' : 'l';
  else dir = dy > 0 ? 'd' : 'u';
  return def[dir] ? dir : 'c';
}

// --- Lean Preview -------------------------------------------------------------
function showPreview(def, dir, x, y) {
  for (const candidate of ['c', 'u', 'l', 'r', 'd']) {
    const member = previewMembers[candidate];
    member.textContent = def[candidate] || '';
    member.hidden = !def[candidate];
  }
  updatePreview(dir, x, y);
  popup.style.display = 'flex';
}

function updatePreview(dir, x, y) {
  for (const candidate of ['c', 'u', 'l', 'r', 'd']) {
    previewMembers[candidate].classList.toggle('is-selected', candidate === dir);
  }
  positionPreview(x, y);
}

function positionPreview(x, y) {
  const width = 132;
  const height = 108;
  const safe = 4;
  const idealLeft = x - width / 2;
  const idealTop = y - height - PREVIEW_GAP;
  popup.style.left = Math.max(safe, Math.min(innerWidth - width - safe, idealLeft)) + 'px';
  popup.style.top = Math.max(safe, Math.min(innerHeight - height - safe, idealTop)) + 'px';
}

function hidePopup() { popup.style.display = 'none'; }

// --- Predictive heatmap -------------------------------------------------------
// At rest, filtered unigram heat points to characters that can start a word.
// During composition, trigram/bigram heat points to the statistically likely
// next character. Only the top few keys carry a probability-scaled ring.
const HEAT_TOP = 6;          // how many key families may glow
const HEAT_FLOOR = 0.02;     // ignore anything below 2% — not worth a hint
const MEMBER_TOP = 8;        // exact-character hints across the glowing families

// Raw unigram frequency is misleading at word start because coeng and dependent
// vowels are globally common but cannot start a Khmer word. Keep only consonants
// (U+1780–U+17A2) and independent vowels (U+17A3–U+17B3).
const START_UNIGRAM = Object.fromEntries(
  Object.entries(UNIGRAM).filter(([glyph]) => {
    if (Array.from(glyph).length !== 1) return false;
    const codepoint = glyph.codePointAt(0);
    return codepoint >= 0x1780 && codepoint <= 0x17b3;
  })
);

// Probability distribution for the current state. Back-off while composing is
// trigram (last 2 glyphs) → bigram (last glyph); idle uses filtered unigram heat.
function heatContext() {
  const g = Array.from(preedit);
  const prev1 = g[g.length - 1];
  const prev2 = g[g.length - 2];
  if (prev2 && prev1 && TRIGRAM[prev2 + prev1]) return { dist: TRIGRAM[prev2 + prev1], isStart: false };
  if (prev1 && BIGRAM[prev1]) return { dist: BIGRAM[prev1], isStart: false };
  if (!prev1) return { dist: START_UNIGRAM, isStart: true };
  return { dist: {}, isStart: false };
}

// Map a probability to a ring intensity in [0,1], scaled so the current max = full.
function heatScale(p, max) {
  if (!max || p < HEAT_FLOOR) return 0;
  return Math.min(1, p / max);
}

// Blue (cold, t=0) → red (hot, t=1) heat ramp: blue → cyan → green → yellow → red.
function heatColor(t) {
  const stops = [
    [0.0, [40, 90, 220]],   // blue
    [0.25, [30, 180, 200]], // cyan
    [0.5, [70, 190, 90]],   // green
    [0.75, [240, 190, 40]], // yellow
    [1.0, [225, 50, 45]],   // red
  ];
  for (let i = 1; i < stops.length; i++) {
    if (t <= stops[i][0]) {
      const [t0, c0] = stops[i - 1], [t1, c1] = stops[i];
      const f = (t - t0) / (t1 - t0);
      const c = c0.map((v, k) => Math.round(v + f * (c1[k] - v)));
      return `rgb(${c[0]},${c[1]},${c[2]})`;
    }
  }
  return 'rgb(225,50,45)';
}

// Apply heat to a key/cell: `t` (0..1) picks the ramp color, `opacity` the glow.
// t<0 clears the ring.
function paintHeat(el, t, opacity) {
  if (t < 0) {
    el.style.setProperty('--heat', '0');
    el.style.setProperty('--heat-color', 'transparent');
    return;
  }
  el.style.setProperty('--heat', opacity.toFixed(2));
  el.style.setProperty('--heat-color', heatColor(t));
}

function paintMemberHeat(el, rankHeat, opacity) {
  if (!el) return;
  if (opacity <= 0) {
    el.style.setProperty('--member-heat', '0');
    el.style.setProperty('--member-heat-color', 'transparent');
    return;
  }
  el.style.setProperty('--member-heat', opacity.toFixed(2));
  el.style.setProperty('--member-heat-color', heatColor(rankHeat));
}

// Paint the resting board: each key glows by its MOST LIKELY glyph (max, not sum).
function updateHeat() {
  const { dist, isStart } = heatContext();
  // score every key by its hottest glyph
  const scored = keyEls.map(el => {
    const def = el.__def;
    let best = 0;
    for (const d of ['c', 'u', 'l', 'r', 'd']) {
      const g = def[d];
      if (g && dist[g] > best) best = dist[g];
    }
    return { el, p: best };
  });
  const max = Math.max(...scored.map(s => s.p), 0);
  // only the top HEAT_TOP above the floor glow
  const glowing = new Set(
    scored.filter(s => s.p >= HEAT_FLOOR).sort((a, b) => b.p - a.p).slice(0, HEAT_TOP).map(s => s.el)
  );
  // Member guidance is deliberately sparse: rank individual characters across
  // the glowing families, then outline only the eight strongest candidates.
  const rankedMembers = keyEls
    .filter(el => glowing.has(el))
    .flatMap(el => ['c', 'u', 'l', 'r', 'd'].map(d => ({ el: el.__members[d], p: dist[el.__def[d]] || 0 })))
    .filter(item => item.el && item.p > 0)
    .sort((a, b) => b.p - a.p)
    .slice(0, MEMBER_TOP);
  const memberRank = new Map(rankedMembers.map((item, rank) => [item.el, rank]));
  for (const { el, p } of scored) {
    if (glowing.has(el)) {
      const t = heatScale(p, max);           // 0..1 → blue..red
      const opacity = isStart ? 0.28 + 0.32 * t : 0.6 + 0.4 * t;
      paintHeat(el, t, opacity);              // idle guidance is softer than prediction
      for (const d of ['c', 'u', 'l', 'r', 'd']) {
        const member = el.__members[d];
        const rank = memberRank.get(member);
        const rankStrength = rank === undefined ? 0 : 1 - (rank / MEMBER_TOP) * 0.6;
        const rankHeat = rank === undefined ? 0 : 1 - rank / Math.max(1, rankedMembers.length - 1);
        paintMemberHeat(member, rankHeat, rankStrength);
      }
    } else {
      paintHeat(el, -1);
      for (const d of ['c', 'u', 'l', 'r', 'd']) paintMemberHeat(el.__members[d], 0, 0);
    }
  }
}

// --- Composition: preedit + commit (IME model) --------------------------------
// Lean selections and marks build an underlined PREEDIT (the word being composed) that is not
// yet in the document. Suggestions rank against the preedit. Enter or a
// suggestion-tap COMMITS it into the committed text and clears the preedit.
// Space commits the raw preedit (if any) then adds a literal separator. This
// mirrors the khmerime engine's Composition -> commit model, and lets the
// suggestions match the bounded word being composed rather than the whole tail.
const committedEl = document.getElementById('committed');
const preeditEl = document.getElementById('preedit');
let committed = '';
let preedit = '';

function render() {
  committedEl.textContent = committed;
  preeditEl.textContent = preedit;
  scrollOut();
  refreshSuggestions();
  updateHeat();
}
function scrollOut() { output.scrollTop = output.scrollHeight; }

// A lean selection / quick-access mark appends to the composing preedit.
function insert(s) { preedit += s; render(); }

// Commit the current preedit (raw) into the document, then clear it.
function commitPreedit() {
  if (!preedit) return;
  committed += preedit;
  preedit = '';
}

// Enter: commit the preedit (no newline in this single-field prototype).
function commitComposition() { commitPreedit(); render(); }

// Space: commit any preedit, then a literal separator into the document.
function space() { commitPreedit(); committed += ' '; render(); }

function backspace() {
  // Backspace edits the preedit first; once it is empty it trims committed text.
  // One Unicode code point at a time (grapheme clusters are a later concern).
  if (preedit) {
    const arr = Array.from(preedit); arr.pop(); preedit = arr.join('');
  } else {
    const arr = Array.from(committed); arr.pop(); committed = arr.join('');
  }
  render();
}

// --- Suggestions (prefix match over WORDS) -----------------------------------
// Khmer words normally run together without spaces. Accepting a suggestion
// records an invisible boundary; the next typed character starts a fresh prefix
// even though the rendered Khmer remains continuous. We intentionally do not
// guess boundaries for fully manual text in this layout prototype.
const sugBar = document.getElementById('suggestions');
const MAX_SUGGESTIONS = 3;
const IDLE_SUGGESTIONS = ['ខ្មែរ', '។', 'ៗ'];

function refreshSuggestions() {
  const prefix = preedit;
  sugBar.innerHTML = '';
  if (!prefix) {
    for (const item of IDLE_SUGGESTIONS) addIdleSuggestion(item);
    return;
  }
  // Offer the raw preedit first so a user can always commit exactly what they
  // typed, even when it is not a dictionary word.
  addSuggestion(prefix, true);
  let n = 0;
  for (const w of WORDS) {
    if (w.length > prefix.length && w.startsWith(prefix)) {
      addSuggestion(w);
      if (++n >= MAX_SUGGESTIONS - 1) break;
    }
  }
}

function addIdleSuggestion(value) {
  const el = document.createElement('button');
  el.type = 'button';
  el.className = 'sug placeholder';
  el.textContent = value;
  el.addEventListener('pointerdown', e => {
    e.preventDefault();
    // An idle suggestion commits directly into the document (no preedit yet).
    committed += value;
    render();
  });
  sugBar.appendChild(el);
}

function addSuggestion(word, isRaw = false) {
  const el = document.createElement('button');
  el.type = 'button';
  el.className = isRaw ? 'sug raw' : 'sug';
  el.textContent = word;
  el.addEventListener('pointerdown', e => {
    e.preventDefault();
    // Tapping a suggestion commits that word and clears the preedit.
    committed += word;
    preedit = '';
    render();
  });
  sugBar.appendChild(el);
}


render();
