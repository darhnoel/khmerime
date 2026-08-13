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
const QUICK_ACCESS_ITEMS = [
  { display: '។', commit: '។', label: 'Khmer full stop' },
  { display: '៕', commit: '៕', label: 'Khmer final period' },
  { display: '៖', commit: '៖', label: 'Khmer sign camnuc pii kuuh' },
  { display: 'ៈ', commit: 'ៈ', label: 'Yukaleapintu' },
  { display: 'ៗ', commit: 'ៗ', label: 'Khmer repetition sign' },
  { display: '៘', commit: '៘' },
  { display: '៙', commit: '៙' },
  { display: '៚', commit: '៚' },
  { display: '៛', commit: '៛', label: 'Khmer currency symbol riel' },
  { display: '◌៉', commit: '៉', label: 'Muusikatoan (sanhya thmenh kandol)' },
  { display: '◌់', commit: '់', label: 'Bantak' },
  { display: '◌៊', commit: '៊', label: 'Triisap' },
  { display: '◌័', commit: '័', label: 'Samyok sannya' },
  { display: '◌៌', commit: '៌' },
  { display: '◌៍', commit: '៍' },
  { display: '◌៏', commit: '៏' },
  { display: '◌៎', commit: '៎' },
  { display: '◌៑', commit: '៑' },
];

// --- Build the keyboard from KEYMAP ------------------------------------------
KEYMAP.forEach(row => {
  const rowEl = document.createElement('div');
  rowEl.className = 'kb-row';
  row.forEach(def => rowEl.appendChild(makeKey(def)));
  kb.appendChild(rowEl);
});

QUICK_ACCESS_ITEMS.forEach(item => quickAccess.appendChild(makeQuickAccessKey(item)));

// Familiar mobile action row. Non-character controls are deliberately inert in
// this layout prototype, but retain pressed feedback so the keyboard feels real.
const actions = document.createElement('div');
actions.className = 'kb-row actions';
actions.appendChild(makeAction('🌐', () => {}, 'icon', 'Switch keyboard'));
actions.appendChild(makeAction('123', () => {}, 'mode', 'Numbers'));
actions.appendChild(makeAction('ដកឃ្លា', space, 'space', 'Space'));
actions.appendChild(makeAction('⏎', commitComposition, 'return', 'Return'));
actions.appendChild(makeAction('⌫', backspace, 'icon', 'Delete'));
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
