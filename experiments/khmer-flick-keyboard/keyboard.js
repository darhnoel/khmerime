// Khmer flick keyboard — interaction (MVP)
// ========================================
// Flick-commit gesture: press a key -> a 5-way popup appears -> drag toward a
// direction -> release commits that glyph. A quick tap (no real drag) commits
// the center. Works with both touch and mouse (mouse = desktop testing).

const output = document.getElementById('output');
const popup = document.getElementById('popup');
const kb = document.getElementById('kb');
const quickAccess = document.getElementById('quick-access');
const cells = {
  c: popup.querySelector('.cell.c'),
  u: popup.querySelector('.cell.u'),
  l: popup.querySelector('.cell.l'),
  r: popup.querySelector('.cell.r'),
  d: popup.querySelector('.cell.d'),
};

const FLICK_THRESHOLD = 22; // px before a drag counts as a direction, not a tap

let active = null; // { key, startX, startY, dir }

// Matches the Android/iOS QuickAccessSpec ordering. Dotted circles are
// display-only in the browser; insertion always uses the raw Khmer mark.
// Ordered by corpus frequency (most-used first), measured on the kmwiki corpus.
const QUICK_ACCESS_ITEMS = [
  { display: 'ឲ្យ', commit: 'ឲ្យ', label: 'Aoy (to give / let)' },                 // common word
  { display: '់', commit: '់', label: 'Bantak' },                                // 1.91%
  { display: '។', commit: '។', label: 'Khmer full stop' },                        // 0.71%
  { display: '៏', commit: '៏', label: 'Ahsda' },                                  // 0.33%
  { display: '៉', commit: '៉', label: 'Muusikatoan (sanhya thmenh kandol)' },     // 0.30%
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
KEYMAP.forEach(row => {
  const rowEl = document.createElement('div');
  rowEl.className = 'kb-row';
  row.forEach(def => {
    const el = makeKey(def);
    keyEls.push(el);
    rowEl.appendChild(el);
  });
  kb.appendChild(rowEl);
});

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
  el.innerHTML = `<span class="center">${def.c}</span>`;
  for (const dir of ['u', 'l', 'r', 'd']) {
    if (def[dir]) {
      const h = document.createElement('span');
      h.className = `hint ${dir}`;
      h.textContent = def[dir];
      el.appendChild(h);
    }
  }
  el.__def = def;
  el.addEventListener('pointerdown', onPress);
  return el;
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
  e.currentTarget.setPointerCapture?.(e.pointerId);
  const def = e.currentTarget.__def;
  active = { def, key: e.currentTarget, startX: e.clientX, startY: e.clientY, dir: 'c' };
  active.key.classList.add('is-pressed');
  showPopup(def, e.currentTarget);
  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onRelease, { once: true });
  window.addEventListener('pointercancel', onCancel, { once: true });
}

function onMove(e) {
  if (!active) return;
  const dir = directionOf(e.clientX - active.startX, e.clientY - active.startY, active.def);
  active.dir = dir;
  highlight(dir);
}

function onRelease() {
  window.removeEventListener('pointermove', onMove);
  window.removeEventListener('pointercancel', onCancel);
  if (active) {
    const glyph = active.def[active.dir];
    if (glyph) insert(glyph);
    active.key.classList.remove('is-pressed');
  }
  hidePopup();
  active = null;
}

function onCancel() {
  window.removeEventListener('pointermove', onMove);
  window.removeEventListener('pointerup', onRelease);
  active?.key.classList.remove('is-pressed');
  hidePopup();
  active = null;
}

// Which direction a drag vector points to — only counts if past the threshold
// AND that direction actually has a glyph; otherwise falls back to center.
function directionOf(dx, dy, def) {
  const dist = Math.hypot(dx, dy);
  if (dist < FLICK_THRESHOLD) return 'c';
  const horizontal = Math.abs(dx) > Math.abs(dy);
  let dir;
  if (horizontal) dir = dx > 0 ? 'r' : 'l';
  else dir = dy > 0 ? 'd' : 'u';
  return def[dir] ? dir : 'c';
}

// --- Popup --------------------------------------------------------------------
function showPopup(def, key) {
  for (const dir of ['c', 'u', 'l', 'r', 'd']) {
    const cell = cells[dir];
    cell.textContent = def[dir] || '';
    cell.classList.toggle('empty', !def[dir]);
    cell.classList.remove('active');
  }
  cells.c.classList.add('active');
  updatePopupHeat(def);
  // Center the cross on the key and clamp it inside the viewport. Keeping the
  // choices visible is especially important for the edge keys on narrow phones.
  const rect = key.getBoundingClientRect();
  const width = 174;
  const height = 174;
  const safe = 4;
  const idealLeft = rect.left + rect.width / 2 - width / 2;
  const idealTop = rect.top + rect.height / 2 - height / 2;
  popup.style.left = Math.max(safe, Math.min(innerWidth - width - safe, idealLeft)) + 'px';
  popup.style.top = Math.max(safe, Math.min(innerHeight - height - safe, idealTop)) + 'px';
  popup.style.display = 'block';
}

function highlight(dir) {
  for (const d of ['c', 'u', 'l', 'r', 'd']) cells[d].classList.toggle('active', d === dir);
}

function hidePopup() { popup.style.display = 'none'; }

// --- Bigram heatmap -----------------------------------------------------------
// After each character, hint at the likely next key from a Khmer bigram model
// (BIGRAM[prev][next] = P(next|prev); UNIGRAM as the no-context fallback). Keys
// carry a glowing ring whose brightness tracks probability; only the top few glow.
const HEAT_TOP = 6;          // how many keys/glyphs may glow
const HEAT_FLOOR = 0.02;     // ignore anything below 2% — not worth a hint

// Probability distribution over next characters, given the current composing state.
// Back-off: trigram (last 2 glyphs) → bigram (last glyph). Returns null when there
// is no real context, so the heatmap shows nothing rather than lighting up the
// globally-common keys (unigram) on every fresh word — that would be noise.
function nextDist() {
  const g = Array.from(preedit);
  const prev1 = g[g.length - 1];
  const prev2 = g[g.length - 2];
  if (prev2 && prev1 && TRIGRAM[prev2 + prev1]) return TRIGRAM[prev2 + prev1];
  if (prev1 && BIGRAM[prev1]) return BIGRAM[prev1];
  return null;
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

// Paint the resting board: each key glows by its MOST LIKELY glyph (max, not sum).
function updateHeat() {
  const dist = nextDist();
  if (!dist) { keyEls.forEach(el => paintHeat(el, -1)); return; }  // no context → no heat
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
  for (const { el, p } of scored) {
    if (glowing.has(el)) {
      const t = heatScale(p, max);           // 0..1 → blue..red
      paintHeat(el, t, 0.6 + 0.4 * t);       // opacity floor so even blue keys show
    } else {
      paintHeat(el, -1);
    }
  }
}

// Paint the flick popup: each direction glows by its own glyph's probability.
function updatePopupHeat(def) {
  const dist = nextDist();
  if (!dist) { ['c', 'u', 'l', 'r', 'd'].forEach(d => paintHeat(cells[d], -1)); return; }
  const vals = ['c', 'u', 'l', 'r', 'd'].map(d => (def[d] && dist[def[d]]) || 0);
  const max = Math.max(...vals, 0);
  ['c', 'u', 'l', 'r', 'd'].forEach((d, i) => {
    if (vals[i] >= HEAT_FLOOR && max) {
      const t = heatScale(vals[i], max);
      paintHeat(cells[d], t, 0.6 + 0.4 * t);
    } else {
      paintHeat(cells[d], -1);
    }
  });
}

// --- Composition: preedit + commit (IME model) --------------------------------
// Flicks/marks build an underlined PREEDIT (the word being composed) that is not
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

// A flick / quick-access mark appends to the composing preedit.
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
